use core::f32::consts::PI;

use macroquad::prelude::*;

mod audio;
mod persistence;
mod ui;

use self::audio::AudioBank;
use self::persistence::{SaveData, load_save_data, store_save_data};
use macroquad::rand::gen_range;

const CURSOR_SPEED: f32 = 720.0;
const ENEMY_BLAST_RADIUS: f32 = 24.0;
const CITY_RESTORE_STEP: u32 = 10_000;

// Screen and difficulty state shared by native and web builds.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScreenState {
    Title,
    Settings,
    Playing,
    GameOver,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Difficulty {
    Cadet,
    Arcade,
    Veteran,
    Mayhem,
}

impl Difficulty {
    const ALL: [Difficulty; 4] = [
        Difficulty::Cadet,
        Difficulty::Arcade,
        Difficulty::Veteran,
        Difficulty::Mayhem,
    ];

    fn label(self) -> &'static str {
        match self {
            Difficulty::Cadet => "CADET",
            Difficulty::Arcade => "ARCADE",
            Difficulty::Veteran => "VETERAN",
            Difficulty::Mayhem => "MAYHEM",
        }
    }

    fn code(self) -> &'static str {
        match self {
            Difficulty::Cadet => "cadet",
            Difficulty::Arcade => "arcade",
            Difficulty::Veteran => "veteran",
            Difficulty::Mayhem => "mayhem",
        }
    }

    fn from_code(code: &str) -> Option<Self> {
        match code {
            "cadet" => Some(Difficulty::Cadet),
            "arcade" => Some(Difficulty::Arcade),
            "veteran" => Some(Difficulty::Veteran),
            "mayhem" => Some(Difficulty::Mayhem),
            _ => None,
        }
    }

    fn description(self) -> &'static str {
        match self {
            Difficulty::Cadet => "slow waves, no smart bombs",
            Difficulty::Arcade => "balanced waves, arcade-first",
            Difficulty::Veteran => "faster missiles, early specials",
            Difficulty::Mayhem => "dense salvos, high smart-bomb rate",
        }
    }

    fn enemy_speed_factor(self) -> f32 {
        match self {
            Difficulty::Cadet => 0.86,
            Difficulty::Arcade => 1.0,
            Difficulty::Veteran => 1.14,
            Difficulty::Mayhem => 1.28,
        }
    }

    fn spawn_factor(self) -> f32 {
        match self {
            Difficulty::Cadet => 1.18,
            Difficulty::Arcade => 1.0,
            Difficulty::Veteran => 0.86,
            Difficulty::Mayhem => 0.72,
        }
    }

    fn wave_density_bonus(self) -> u32 {
        match self {
            Difficulty::Cadet => 0,
            Difficulty::Arcade => 1,
            Difficulty::Veteran => 3,
            Difficulty::Mayhem => 5,
        }
    }

    fn smart_bomb_start_wave(self) -> u32 {
        match self {
            Difficulty::Cadet => 8,
            Difficulty::Arcade => 6,
            Difficulty::Veteran => 5,
            Difficulty::Mayhem => 4,
        }
    }

    fn smart_bomb_chance(self) -> f32 {
        match self {
            Difficulty::Cadet => 0.04,
            Difficulty::Arcade => 0.09,
            Difficulty::Veteran => 0.15,
            Difficulty::Mayhem => 0.24,
        }
    }

    fn air_support_start_wave(self) -> u32 {
        match self {
            Difficulty::Cadet => 3,
            Difficulty::Arcade => 2,
            Difficulty::Veteran => 2,
            Difficulty::Mayhem => 2,
        }
    }

    fn air_support_count_bonus(self) -> u32 {
        match self {
            Difficulty::Cadet => 0,
            Difficulty::Arcade => 1,
            Difficulty::Veteran => 2,
            Difficulty::Mayhem => 3,
        }
    }

    fn next(self, delta: i32) -> Difficulty {
        let index = Self::ALL
            .iter()
            .position(|entry| *entry == self)
            .unwrap_or(1) as i32;
        let next = (index + delta).rem_euclid(Self::ALL.len() as i32) as usize;
        Self::ALL[next]
    }
}

// Runtime-adjustable options that are persisted across launches.
#[derive(Clone, Copy)]
struct GameConfig {
    ammo_per_base: i32,
    blast_radius: f32,
    difficulty: Difficulty,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            ammo_per_base: 10,
            blast_radius: 58.0,
            difficulty: Difficulty::Arcade,
        }
    }
}

impl GameConfig {
    fn restore_defaults(&mut self) {
        *self = Self::default();
    }

    fn clamp(&mut self) {
        self.ammo_per_base = self.ammo_per_base.clamp(4, 20);
        self.blast_radius = self.blast_radius.clamp(32.0, 96.0);
    }

    fn encode(self) -> String {
        format!(
            "ammo={}\nblast={:.0}\ndifficulty={}",
            self.ammo_per_base,
            self.blast_radius,
            self.difficulty.code()
        )
    }

    fn decode(raw: &str) -> Option<Self> {
        let mut config = Self::default();
        let mut touched = false;

        for line in raw.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "ammo" => {
                    config.ammo_per_base = value.trim().parse().ok()?;
                    touched = true;
                }
                "blast" => {
                    config.blast_radius = value.trim().parse().ok()?;
                    touched = true;
                }
                "difficulty" => {
                    config.difficulty = Difficulty::from_code(value.trim())?;
                    touched = true;
                }
                _ => {}
            }
        }

        if touched {
            config.clamp();
            Some(config)
        } else {
            None
        }
    }
}

// Dynamic layout recalculated from the current viewport size.
#[derive(Clone, Copy)]
struct Layout {
    screen: Vec2,
    horizon_y: f32,
    ground_y: f32,
    base_positions: [Vec2; 3],
    city_positions: [Vec2; 6],
}

impl Layout {
    fn new(screen: Vec2) -> Self {
        let ground_y = screen.y * 0.88;
        let horizon_y = screen.y * 0.18;
        let city_y = ground_y - screen.y * 0.022;
        let base_y = ground_y - screen.y * 0.032;

        let base_positions = [
            vec2(screen.x * 0.18, base_y),
            vec2(screen.x * 0.50, base_y),
            vec2(screen.x * 0.82, base_y),
        ];

        let city_positions = [
            vec2(screen.x * 0.08, city_y),
            vec2(screen.x * 0.27, city_y),
            vec2(screen.x * 0.39, city_y),
            vec2(screen.x * 0.61, city_y),
            vec2(screen.x * 0.73, city_y),
            vec2(screen.x * 0.92, city_y),
        ];

        Self {
            screen,
            horizon_y,
            ground_y,
            base_positions,
            city_positions,
        }
    }

    fn cursor_bounds(&self) -> Rect {
        Rect::new(
            36.0,
            36.0,
            (self.screen.x - 72.0).max(1.0),
            (self.ground_y - 84.0).max(1.0),
        )
    }
}

// Core world state.
#[derive(Clone, Copy)]
struct Base {
    position: Vec2,
    ammo: i32,
    alive: bool,
}

#[derive(Clone, Copy)]
struct City {
    position: Vec2,
    alive: bool,
}

#[derive(Clone, Copy)]
struct Star {
    position: Vec2,
    radius: f32,
    alpha: f32,
}

#[derive(Clone, Copy)]
enum SiteKind {
    Base,
    City,
}

#[derive(Clone, Copy)]
struct TargetSlot {
    kind: SiteKind,
    index: usize,
}

#[derive(Clone, Copy)]
enum EnemyKind {
    Basic,
    Splitter {
        split_progress: f32,
    },
    SmartBomb {
        dodges_left: u8,
        cooldown: f32,
        weave_phase: f32,
    },
}

struct PlayerMissile {
    position: Vec2,
    target: Vec2,
    speed: f32,
    color: Color,
}

struct EnemyMissile {
    position: Vec2,
    start: Vec2,
    target: Vec2,
    target_slot: TargetSlot,
    speed: f32,
    kind: EnemyKind,
    color: Color,
}

struct Bomber {
    position: Vec2,
    velocity: Vec2,
    drop_timer: f32,
    wobble: f32,
}

struct Satellite {
    position: Vec2,
    velocity: Vec2,
    drop_timer: f32,
    phase: f32,
}

#[derive(Clone, Copy)]
enum ExplosionOwner {
    Player,
    Enemy,
}

struct Explosion {
    position: Vec2,
    radius: f32,
    max_radius: f32,
    expand_speed: f32,
    contract_speed: f32,
    expanding: bool,
    owner: ExplosionOwner,
}

// Central game state.
//
// This file is intentionally monolithic for now. If further changes accumulate,
// the safest split points are: save/config, audio synthesis, spawning,
// rendering, and UI screens.
pub struct Game {
    layout: Layout,
    stars: Vec<Star>,
    audio: AudioBank,
    mode: ScreenState,
    settings_return: ScreenState,
    settings_cursor: usize,
    config: GameConfig,
    cursor: Vec2,
    last_mouse_position: Option<Vec2>,
    bases: [Base; 3],
    cities: [City; 6],
    player_missiles: Vec<PlayerMissile>,
    enemy_missiles: Vec<EnemyMissile>,
    bombers: Vec<Bomber>,
    satellites: Vec<Satellite>,
    explosions: Vec<Explosion>,
    score: u32,
    high_score: u32,
    next_city_restore_score: u32,
    wave: u32,
    enemies_to_spawn: u32,
    enemies_spawned: u32,
    spawn_timer: f32,
    intermission_timer: f32,
    wave_banner_timer: f32,
    air_support_remaining: u32,
    air_support_cooldown: f32,
    next_air_support_satellite: bool,
    paused: bool,
}

impl Game {
    // Frame update and state transitions.
    pub async fn new(screen: Vec2) -> Self {
        let layout = Layout::new(screen);
        let cursor = vec2(screen.x * 0.5, screen.y * 0.38);
        let save_data = load_save_data();
        let mut game = Self {
            layout,
            stars: make_stars(screen),
            audio: AudioBank::load().await,
            mode: ScreenState::Title,
            settings_return: ScreenState::Title,
            settings_cursor: 0,
            config: save_data.config,
            cursor,
            last_mouse_position: None,
            bases: [Base {
                position: vec2(0.0, 0.0),
                ammo: 0,
                alive: true,
            }; 3],
            cities: [City {
                position: vec2(0.0, 0.0),
                alive: true,
            }; 6],
            player_missiles: Vec::new(),
            enemy_missiles: Vec::new(),
            bombers: Vec::new(),
            satellites: Vec::new(),
            explosions: Vec::new(),
            score: 0,
            high_score: save_data.high_score,
            next_city_restore_score: CITY_RESTORE_STEP,
            wave: 1,
            enemies_to_spawn: 0,
            enemies_spawned: 0,
            spawn_timer: 0.0,
            intermission_timer: 0.0,
            wave_banner_timer: 0.0,
            air_support_remaining: 0,
            air_support_cooldown: 0.0,
            next_air_support_satellite: false,
            paused: false,
        };
        game.populate_defense_line();
        game
    }

    pub fn update(&mut self, dt: f32) -> bool {
        self.sync_layout();
        self.audio.ensure_music();

        if is_key_pressed(KeyCode::F) {
            self.request_fullscreen();
        }

        if is_key_pressed(KeyCode::Q) {
            return self.handle_quit_request();
        }

        match self.mode {
            ScreenState::Title => self.update_title(),
            ScreenState::Settings => self.update_settings(),
            ScreenState::Playing => self.update_playing(dt),
            ScreenState::GameOver => self.update_game_over(),
        }

        false
    }
    fn handle_quit_request(&mut self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            match self.mode {
                ScreenState::Settings => self.mode = self.settings_return,
                ScreenState::Playing | ScreenState::GameOver => self.return_to_title(),
                ScreenState::Title => {}
            }
            false
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            true
        }
    }

    fn request_fullscreen(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            set_fullscreen(true);
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn return_to_title(&mut self) {
        self.mode = ScreenState::Title;
        self.paused = false;
        self.score = 0;
        self.next_city_restore_score = CITY_RESTORE_STEP;
        self.wave = 1;
        self.enemies_to_spawn = 0;
        self.enemies_spawned = 0;
        self.spawn_timer = 0.0;
        self.intermission_timer = 0.0;
        self.wave_banner_timer = 0.0;
        self.player_missiles.clear();
        self.enemy_missiles.clear();
        self.bombers.clear();
        self.satellites.clear();
        self.explosions.clear();
        self.populate_defense_line();
    }

    fn update_title(&mut self) {
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
            self.audio.play_start();
            self.start_campaign();
        }

        if is_key_pressed(KeyCode::O) || is_key_pressed(KeyCode::S) {
            self.settings_return = ScreenState::Title;
            self.settings_cursor = 0;
            self.mode = ScreenState::Settings;
            self.audio.play_ui_move();
        }
    }

    fn update_settings(&mut self) {
        if is_key_pressed(KeyCode::Escape) {
            self.mode = self.settings_return;
            self.audio.play_ui_move();
            return;
        }

        if is_key_pressed(KeyCode::Up) {
            self.settings_cursor = self.settings_cursor.saturating_sub(1);
            self.audio.play_ui_move();
        }
        if is_key_pressed(KeyCode::Down) {
            self.settings_cursor = (self.settings_cursor + 1).min(3);
            self.audio.play_ui_move();
        }
        if is_key_pressed(KeyCode::Left) {
            self.adjust_setting(-1);
        }
        if is_key_pressed(KeyCode::Right) {
            self.adjust_setting(1);
        }
        if is_key_pressed(KeyCode::R) {
            self.config.restore_defaults();
            self.persist_save();
            self.audio.play_ui_move();
        }
        if is_key_pressed(KeyCode::Enter) && self.settings_cursor == 3 {
            self.mode = self.settings_return;
            self.audio.play_ui_move();
        }
    }

    fn adjust_setting(&mut self, delta: i32) {
        match self.settings_cursor {
            0 => self.config.ammo_per_base += delta,
            1 => self.config.blast_radius += delta as f32 * 4.0,
            2 => self.config.difficulty = self.config.difficulty.next(delta),
            _ => return,
        }
        self.config.clamp();
        self.persist_save();
        self.audio.play_ui_move();
    }

    fn update_game_over(&mut self) {
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::R) {
            self.audio.play_start();
            self.start_campaign();
        }
        if is_key_pressed(KeyCode::T) {
            self.mode = ScreenState::Title;
            self.audio.play_ui_move();
        }
        if is_key_pressed(KeyCode::O) || is_key_pressed(KeyCode::S) {
            self.settings_return = ScreenState::GameOver;
            self.settings_cursor = 0;
            self.mode = ScreenState::Settings;
            self.audio.play_ui_move();
        }
    }

    fn update_playing(&mut self, dt: f32) {
        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::P) {
            self.paused = !self.paused;
            self.audio.play_ui_move();
        }

        self.update_cursor(dt);
        self.handle_fire_input();

        if self.paused {
            return;
        }

        if self.wave_banner_timer > 0.0 {
            self.wave_banner_timer = (self.wave_banner_timer - dt).max(0.0);
        }

        if self.intermission_timer > 0.0 {
            self.intermission_timer -= dt;
            if self.intermission_timer <= 0.0 {
                self.wave += 1;
                self.begin_wave();
            }
            return;
        }

        self.update_player_missiles(dt);
        self.update_enemy_missiles(dt);
        self.update_bombers(dt);
        self.update_satellites(dt);
        self.update_explosions(dt);
        self.handle_explosion_hits();
        self.handle_airborne_hits();
        self.update_wave_spawning(dt);

        if self.enemies_spawned == self.enemies_to_spawn
            && self.enemy_missiles.is_empty()
            && self.bombers.is_empty()
            && self.satellites.is_empty()
            && self.intermission_timer <= 0.0
        {
            self.finish_wave();
        }
    }

    fn start_campaign(&mut self) {
        self.config.clamp();
        self.mode = ScreenState::Playing;
        self.paused = false;
        self.score = 0;
        self.next_city_restore_score = CITY_RESTORE_STEP;
        self.wave = 1;
        self.player_missiles.clear();
        self.enemy_missiles.clear();
        self.bombers.clear();
        self.satellites.clear();
        self.explosions.clear();

        for (index, base) in self.bases.iter_mut().enumerate() {
            *base = Base {
                position: self.layout.base_positions[index],
                ammo: self.config.ammo_per_base,
                alive: true,
            };
        }
        for (index, city) in self.cities.iter_mut().enumerate() {
            *city = City {
                position: self.layout.city_positions[index],
                alive: true,
            };
        }

        self.begin_wave();
    }

    fn populate_defense_line(&mut self) {
        for (index, base) in self.bases.iter_mut().enumerate() {
            *base = Base {
                position: self.layout.base_positions[index],
                ammo: self.config.ammo_per_base,
                alive: true,
            };
        }
        for (index, city) in self.cities.iter_mut().enumerate() {
            *city = City {
                position: self.layout.city_positions[index],
                alive: true,
            };
        }
    }

    fn begin_wave(&mut self) {
        self.enemies_to_spawn = 12 + self.wave * 2 + self.config.difficulty.wave_density_bonus();
        self.enemies_spawned = 0;
        self.spawn_timer = 1.0;
        self.intermission_timer = 0.0;
        self.wave_banner_timer = 1.75;
        self.air_support_remaining = self.air_support_quota_for_wave();
        self.air_support_cooldown = 1.2;
        self.next_air_support_satellite = self.wave % 2 == 0;
        self.player_missiles.clear();
        self.enemy_missiles.clear();
        self.bombers.clear();
        self.satellites.clear();
        self.explosions.clear();

        for base in &mut self.bases {
            if base.alive {
                base.ammo = self.config.ammo_per_base;
            }
        }
    }

    fn finish_wave(&mut self) {
        let saved_cities = self.living_city_count() as u32;
        let unused_missiles: u32 = self
            .bases
            .iter()
            .filter(|base| base.alive)
            .map(|base| base.ammo.max(0) as u32)
            .sum();

        self.award_points(saved_cities * 100);
        self.award_points(unused_missiles * 5);
        self.intermission_timer = 2.8;
    }

    fn persist_save(&self) {
        store_save_data(SaveData {
            config: self.config,
            high_score: self.high_score.max(self.score),
        });
    }

    fn air_support_quota_for_wave(&self) -> u32 {
        if self.wave < self.config.difficulty.air_support_start_wave() {
            return 0;
        }

        let base = match self.wave {
            0..=2 => 0,
            3..=4 => 1,
            5..=7 => 2,
            _ => 3,
        };
        base + self.config.difficulty.air_support_count_bonus()
    }

    fn sync_layout(&mut self) {
        let screen = vec2(screen_width(), screen_height());
        if self.layout.screen.distance_squared(screen) < 1.0 {
            return;
        }

        self.layout = Layout::new(screen);
        self.stars = make_stars(screen);
        for (index, base) in self.bases.iter_mut().enumerate() {
            base.position = self.layout.base_positions[index];
        }
        for (index, city) in self.cities.iter_mut().enumerate() {
            city.position = self.layout.city_positions[index];
        }
        let bounds = self.layout.cursor_bounds();
        self.cursor.x = self.cursor.x.clamp(bounds.x, bounds.x + bounds.w);
        self.cursor.y = self.cursor.y.clamp(bounds.y, bounds.y + bounds.h);
    }

    fn update_cursor(&mut self, dt: f32) {
        let mut next = self.cursor;
        let mouse = vec2(mouse_position().0, mouse_position().1);

        if self
            .last_mouse_position
            .map(|last| last.distance_squared(mouse) > 1.0)
            .unwrap_or(true)
        {
            next = mouse;
        }
        self.last_mouse_position = Some(mouse);

        let mut axis = vec2(0.0, 0.0);
        if is_key_down(KeyCode::Left) {
            axis.x -= 1.0;
        }
        if is_key_down(KeyCode::Right) {
            axis.x += 1.0;
        }
        if is_key_down(KeyCode::Up) {
            axis.y -= 1.0;
        }
        if is_key_down(KeyCode::Down) {
            axis.y += 1.0;
        }
        if axis.length_squared() > 0.0 {
            next += axis.normalize() * CURSOR_SPEED * dt;
        }

        let bounds = self.layout.cursor_bounds();
        next.x = next.x.clamp(bounds.x, bounds.x + bounds.w);
        next.y = next.y.clamp(bounds.y, bounds.y + bounds.h);
        self.cursor = next;
    }

    fn handle_fire_input(&mut self) {
        if self.mode != ScreenState::Playing || self.paused || self.intermission_timer > 0.0 {
            return;
        }

        if is_key_pressed(KeyCode::Z) {
            self.launch_player_missile(0);
        }
        if is_key_pressed(KeyCode::X) {
            self.launch_player_missile(1);
        }
        if is_key_pressed(KeyCode::C) {
            self.launch_player_missile(2);
        }
    }

    fn launch_player_missile(&mut self, base_index: usize) {
        let Some(base) = self.bases.get_mut(base_index) else {
            return;
        };
        if !base.alive || base.ammo <= 0 {
            return;
        }

        base.ammo -= 1;
        let speed = if base_index == 1 { 860.0 } else { 710.0 };
        let color = match base_index {
            0 => color_u8!(64, 200, 255, 255),
            1 => color_u8!(120, 255, 210, 255),
            _ => color_u8!(255, 220, 96, 255),
        };

        self.player_missiles.push(PlayerMissile {
            position: base.position,
            target: self.cursor,
            speed,
            color,
        });
        self.audio.play_fire();
    }

    fn update_player_missiles(&mut self, dt: f32) {
        let mut detonations = Vec::new();
        self.player_missiles.retain_mut(|missile| {
            let to_target = missile.target - missile.position;
            let step = missile.speed * dt;
            if to_target.length() <= step {
                missile.position = missile.target;
                detonations.push(Explosion {
                    position: missile.position,
                    radius: 4.0,
                    max_radius: self.config.blast_radius,
                    expand_speed: 180.0,
                    contract_speed: 90.0,
                    expanding: true,
                    owner: ExplosionOwner::Player,
                });
                false
            } else {
                missile.position += to_target.normalize() * step;
                true
            }
        });
        if !detonations.is_empty() {
            self.audio.play_explosion();
        }
        self.explosions.extend(detonations);
    }

    fn update_enemy_missiles(&mut self, dt: f32) {
        let mut spawned_children = Vec::new();
        let mut detonations = Vec::new();
        let mut destroyed_sites = Vec::new();
        let available_targets = self.available_target_slots();
        let bases = self.bases;
        let cities = self.cities;
        let player_blasts: Vec<(Vec2, f32)> = self
            .explosions
            .iter()
            .filter(|explosion| matches!(explosion.owner, ExplosionOwner::Player))
            .map(|explosion| (explosion.position, explosion.radius))
            .collect();
        let mut smart_dodge_triggered = false;
        let layout = self.layout;

        self.enemy_missiles.retain_mut(|missile| {
            if let EnemyKind::SmartBomb {
                dodges_left,
                cooldown,
                weave_phase,
            } = &mut missile.kind
            {
                *cooldown = (*cooldown - dt).max(0.0);
                *weave_phase += dt * 7.5;
                let smart_course = (missile.target - missile.position).normalize_or_zero();
                let mut smart_side = vec2(-smart_course.y, smart_course.x);
                if smart_side.length_squared() == 0.0 {
                    smart_side = vec2(1.0, 0.0);
                }
                missile.position += smart_side.normalize() * weave_phase.sin() * 34.0 * dt;
                missile.position.x = missile.position.x.clamp(28.0, layout.screen.x - 28.0);
                missile.position.y = missile
                    .position
                    .y
                    .clamp(layout.horizon_y, layout.ground_y - 86.0);

                if *dodges_left > 0 && *cooldown <= 0.0 {
                    if let Some((blast_pos, blast_radius)) =
                        nearest_blast(missile.position, &player_blasts)
                    {
                        if missile.position.distance(blast_pos) <= blast_radius + 34.0 {
                            let sign = if smart_side.dot(blast_pos - missile.position) > 0.0 {
                                -1.0
                            } else {
                                1.0
                            };
                            missile.position += smart_side.normalize() * sign * 190.0 * dt;
                            missile.position.x =
                                missile.position.x.clamp(28.0, layout.screen.x - 28.0);
                            missile.position.y = missile
                                .position
                                .y
                                .clamp(layout.horizon_y, layout.ground_y - 86.0);
                            missile.start = missile.position;
                            *dodges_left -= 1;
                            *cooldown = 0.22;
                            smart_dodge_triggered = true;
                        }
                    }
                }
            }

            let mut should_keep = true;
            let to_target = missile.target - missile.position;
            let step = missile.speed * dt;

            if let EnemyKind::Splitter { split_progress } = missile.kind {
                let travel = missile.start.distance(missile.position);
                let full = missile.start.distance(missile.target).max(1.0);
                if travel / full >= split_progress {
                    for _ in 0..2 {
                        if let Some(slot) = pick_random_slot(&available_targets) {
                            let target = slot_position(slot, &bases, &cities);
                            spawned_children.push(EnemyMissile {
                                position: missile.position,
                                start: missile.position,
                                target,
                                target_slot: slot,
                                speed: missile.speed * 1.12,
                                kind: EnemyKind::Basic,
                                color: color_u8!(255, 120, 92, 255),
                            });
                        }
                    }
                    should_keep = false;
                }
            }

            if !should_keep {
                return false;
            }

            if to_target.length() <= step {
                missile.position = missile.target;
                destroyed_sites.push(missile.target_slot);
                detonations.push(Explosion {
                    position: missile.position,
                    radius: 6.0,
                    max_radius: ENEMY_BLAST_RADIUS,
                    expand_speed: 120.0,
                    contract_speed: 70.0,
                    expanding: true,
                    owner: ExplosionOwner::Enemy,
                });
                false
            } else {
                missile.position += to_target.normalize() * step;
                true
            }
        });

        if smart_dodge_triggered {
            self.audio.play_smart_bomb();
        }
        if !detonations.is_empty() {
            self.audio.play_explosion();
        }

        self.enemy_missiles.extend(spawned_children);
        self.explosions.extend(detonations);

        for slot in destroyed_sites {
            self.destroy_site(slot);
        }
    }

    fn update_bombers(&mut self, dt: f32) {
        let mut drops = Vec::new();
        self.bombers.retain_mut(|bomber| {
            bomber.wobble += dt * 2.8;
            bomber.position += bomber.velocity * dt;
            bomber.position.y += bomber.wobble.sin() * 10.0 * dt;
            bomber.drop_timer -= dt;

            let on_screen =
                bomber.position.x >= -90.0 && bomber.position.x <= self.layout.screen.x + 90.0;
            if on_screen && bomber.drop_timer <= 0.0 {
                drops.push(bomber.position);
                bomber.drop_timer = gen_range(0.8, 1.35);
            }

            bomber.position.x >= -140.0 && bomber.position.x <= self.layout.screen.x + 140.0
        });

        for drop in drops {
            let burst = if self.wave >= 8 {
                3
            } else if self.wave >= 5 {
                2
            } else {
                1
            };
            for _ in 0..burst {
                self.spawn_air_dropped_missile(drop, 1.02, 0.0, 0.12);
            }
        }
    }

    fn update_satellites(&mut self, dt: f32) {
        let mut drops = Vec::new();
        self.satellites.retain_mut(|satellite| {
            satellite.phase += dt * 4.2;
            satellite.position += satellite.velocity * dt;
            satellite.position.y += satellite.phase.sin() * 6.0 * dt;
            satellite.drop_timer -= dt;

            let on_screen = satellite.position.x >= -50.0
                && satellite.position.x <= self.layout.screen.x + 50.0;
            if on_screen && satellite.drop_timer <= 0.0 {
                drops.push(satellite.position);
                satellite.drop_timer = gen_range(0.55, 0.95);
            }

            satellite.position.x >= -100.0 && satellite.position.x <= self.layout.screen.x + 100.0
        });

        for drop in drops {
            let burst = if self.wave >= 7 { 2 } else { 1 };
            for _ in 0..burst {
                self.spawn_air_dropped_missile(
                    drop,
                    1.14,
                    self.config.difficulty.smart_bomb_chance() * 0.55,
                    0.0,
                );
            }
        }
    }

    fn update_explosions(&mut self, dt: f32) {
        self.explosions.retain_mut(|explosion| {
            if explosion.expanding {
                explosion.radius += explosion.expand_speed * dt;
                if explosion.radius >= explosion.max_radius {
                    explosion.radius = explosion.max_radius;
                    explosion.expanding = false;
                }
            } else {
                explosion.radius -= explosion.contract_speed * dt;
            }
            explosion.radius > 0.5
        });
    }

    fn handle_explosion_hits(&mut self) {
        let mut score_events = 0u32;
        let mut detonations = Vec::new();

        self.enemy_missiles.retain(|missile| {
            let hit = self.explosions.iter().any(|explosion| {
                matches!(explosion.owner, ExplosionOwner::Player)
                    && explosion.position.distance(missile.position) <= explosion.radius
            });

            if hit {
                score_events += enemy_score(missile.kind);
                detonations.push(Explosion {
                    position: missile.position,
                    radius: 6.0,
                    max_radius: 22.0,
                    expand_speed: 150.0,
                    contract_speed: 100.0,
                    expanding: true,
                    owner: ExplosionOwner::Enemy,
                });
                false
            } else {
                true
            }
        });

        if score_events > 0 {
            self.award_points(score_events);
            self.audio.play_explosion();
        }
        self.explosions.extend(detonations);
    }

    fn handle_airborne_hits(&mut self) {
        let blast_fields: Vec<(Vec2, f32)> = self
            .explosions
            .iter()
            .filter(|explosion| matches!(explosion.owner, ExplosionOwner::Player))
            .map(|explosion| (explosion.position, explosion.radius))
            .collect();

        let mut score = 0u32;
        let mut detonations = Vec::new();
        let mut smart_sound = false;

        self.bombers.retain(|bomber| {
            let hit = blast_fields
                .iter()
                .any(|(position, radius)| bomber.position.distance(*position) <= *radius + 24.0);
            if hit {
                score += 100;
                detonations.push(Explosion {
                    position: bomber.position,
                    radius: 10.0,
                    max_radius: 26.0,
                    expand_speed: 180.0,
                    contract_speed: 110.0,
                    expanding: true,
                    owner: ExplosionOwner::Enemy,
                });
                false
            } else {
                true
            }
        });

        self.satellites.retain(|satellite| {
            let hit = blast_fields
                .iter()
                .any(|(position, radius)| satellite.position.distance(*position) <= *radius + 16.0);
            if hit {
                score += 100;
                smart_sound = true;
                detonations.push(Explosion {
                    position: satellite.position,
                    radius: 8.0,
                    max_radius: 20.0,
                    expand_speed: 210.0,
                    contract_speed: 120.0,
                    expanding: true,
                    owner: ExplosionOwner::Enemy,
                });
                false
            } else {
                true
            }
        });

        if score > 0 {
            self.award_points(score);
            self.audio.play_explosion();
        }
        if smart_sound {
            self.audio.play_smart_bomb();
        }
        self.explosions.extend(detonations);
    }

    fn update_wave_spawning(&mut self, dt: f32) {
        if self.enemies_spawned >= self.enemies_to_spawn {
            self.maybe_spawn_air_support(dt);
            return;
        }

        self.spawn_timer -= dt;
        while self.spawn_timer <= 0.0 && self.enemies_spawned < self.enemies_to_spawn {
            self.spawn_enemy_missile();
            self.enemies_spawned += 1;
            self.spawn_timer += self.next_spawn_interval();
        }
        self.maybe_spawn_air_support(dt);
    }

    fn spawn_enemy_missile(&mut self) {
        let Some(target_slot) = self.random_target_slot() else {
            self.trigger_game_over();
            return;
        };

        let start = vec2(
            gen_range(48.0, self.layout.screen.x - 48.0),
            self.layout.horizon_y,
        );
        let base_speed =
            (74.0 + self.wave as f32 * 9.0) * self.config.difficulty.enemy_speed_factor();
        let smart_bombs_allowed = self.wave >= self.config.difficulty.smart_bomb_start_wave();
        let roll = gen_range(0.0, 1.0);
        let kind = if smart_bombs_allowed && roll < self.config.difficulty.smart_bomb_chance() {
            EnemyKind::SmartBomb {
                dodges_left: 3,
                cooldown: 0.0,
                weave_phase: gen_range(0.0, PI * 2.0),
            }
        } else if self.wave >= 3 && roll < 0.22 + self.config.difficulty.smart_bomb_chance() * 0.4 {
            EnemyKind::Splitter {
                split_progress: gen_range(0.33, 0.68),
            }
        } else {
            EnemyKind::Basic
        };
        self.spawn_targeted_enemy(start, target_slot, base_speed, kind);
    }

    fn maybe_spawn_air_support(&mut self, dt: f32) {
        if self.air_support_remaining == 0 {
            return;
        }
        if self.wave < self.config.difficulty.air_support_start_wave() {
            return;
        }
        if !self.bombers.is_empty() || !self.satellites.is_empty() {
            return;
        }

        self.air_support_cooldown -= dt;
        if self.air_support_cooldown > 0.0 {
            return;
        }

        if self.next_air_support_satellite && self.wave >= 4 {
            self.spawn_satellite();
        } else {
            self.spawn_bomber();
        }
        self.next_air_support_satellite = !self.next_air_support_satellite;
        self.air_support_remaining = self.air_support_remaining.saturating_sub(1);
        self.air_support_cooldown = (2.0 - self.wave as f32 * 0.05).clamp(0.75, 2.0);
    }

    fn spawn_bomber(&mut self) {
        let direction = if gen_range(0, 2) == 0 { 1.0 } else { -1.0 };
        let start_x = if direction > 0.0 {
            -90.0
        } else {
            self.layout.screen.x + 90.0
        };
        self.bombers.push(Bomber {
            position: vec2(start_x, self.layout.screen.y * 0.46),
            velocity: vec2(direction * (104.0 + self.wave as f32 * 3.5), 0.0),
            drop_timer: 0.95,
            wobble: gen_range(0.0, PI * 2.0),
        });
    }

    fn spawn_satellite(&mut self) {
        let direction = if gen_range(0, 2) == 0 { 1.0 } else { -1.0 };
        let start_x = if direction > 0.0 {
            -50.0
        } else {
            self.layout.screen.x + 50.0
        };
        self.satellites.push(Satellite {
            position: vec2(start_x, self.layout.screen.y * 0.24),
            velocity: vec2(direction * (190.0 + self.wave as f32 * 7.0), 0.0),
            drop_timer: 0.62,
            phase: gen_range(0.0, PI * 2.0),
        });
    }

    fn spawn_air_dropped_missile(
        &mut self,
        start: Vec2,
        speed_scale: f32,
        smart_bias: f32,
        splitter_bias: f32,
    ) {
        let Some(target_slot) = self.random_target_slot() else {
            return;
        };

        let allow_smart = self.wave >= self.config.difficulty.smart_bomb_start_wave();
        let roll = gen_range(0.0, 1.0);
        let kind = if allow_smart && roll < smart_bias {
            EnemyKind::SmartBomb {
                dodges_left: 2,
                cooldown: 0.0,
                weave_phase: gen_range(0.0, PI * 2.0),
            }
        } else if self.wave >= 4 && roll < smart_bias + splitter_bias {
            EnemyKind::Splitter {
                split_progress: gen_range(0.42, 0.72),
            }
        } else {
            EnemyKind::Basic
        };

        let speed = (86.0 + self.wave as f32 * 8.0)
            * self.config.difficulty.enemy_speed_factor()
            * speed_scale;
        self.spawn_targeted_enemy(start, target_slot, speed, kind);
    }

    fn spawn_targeted_enemy(
        &mut self,
        start: Vec2,
        target_slot: TargetSlot,
        mut speed: f32,
        kind: EnemyKind,
    ) {
        let target = self.target_position(target_slot);
        if matches!(kind, EnemyKind::SmartBomb { .. }) {
            speed *= 1.14;
        }
        let color = match kind {
            EnemyKind::Basic => color_u8!(255, 94, 80, 255),
            EnemyKind::Splitter { .. } => color_u8!(255, 64, 200, 255),
            EnemyKind::SmartBomb { .. } => color_u8!(255, 255, 110, 255),
        };

        self.enemy_missiles.push(EnemyMissile {
            position: start,
            start,
            target,
            target_slot,
            speed,
            kind,
            color,
        });
    }

    fn next_spawn_interval(&self) -> f32 {
        let base = (0.96 - self.wave as f32 * 0.03).clamp(0.14, 0.96);
        (base * self.config.difficulty.spawn_factor()).clamp(0.1, 1.2)
    }

    fn random_target_slot(&self) -> Option<TargetSlot> {
        let slots = self.available_target_slots();
        pick_random_slot(&slots)
    }

    fn available_target_slots(&self) -> Vec<TargetSlot> {
        let mut slots = Vec::new();
        for (index, city) in self.cities.iter().enumerate() {
            if city.alive {
                slots.push(TargetSlot {
                    kind: SiteKind::City,
                    index,
                });
            }
        }
        for (index, base) in self.bases.iter().enumerate() {
            if base.alive {
                slots.push(TargetSlot {
                    kind: SiteKind::Base,
                    index,
                });
            }
        }
        slots
    }

    fn target_position(&self, slot: TargetSlot) -> Vec2 {
        match slot.kind {
            SiteKind::Base => self.bases[slot.index].position,
            SiteKind::City => self.cities[slot.index].position,
        }
    }

    fn destroy_site(&mut self, slot: TargetSlot) {
        match slot.kind {
            SiteKind::Base => {
                if let Some(base) = self.bases.get_mut(slot.index) {
                    base.alive = false;
                    base.ammo = 0;
                }
            }
            SiteKind::City => {
                if let Some(city) = self.cities.get_mut(slot.index) {
                    city.alive = false;
                }
            }
        }

        if self.living_city_count() == 0 {
            self.trigger_game_over();
        }
    }

    fn trigger_game_over(&mut self) {
        if self.score > self.high_score {
            self.high_score = self.score;
            self.persist_save();
        }
        self.mode = ScreenState::GameOver;
        self.paused = false;
        self.player_missiles.clear();
        self.enemy_missiles.clear();
        self.bombers.clear();
        self.satellites.clear();
        self.audio.play_game_over();
    }

    fn living_city_count(&self) -> usize {
        self.cities.iter().filter(|city| city.alive).count()
    }

    fn award_points(&mut self, base_points: u32) {
        let points = base_points * self.score_multiplier();
        self.score = self.score.saturating_add(points);
        if self.score > self.high_score {
            self.high_score = self.score;
            self.persist_save();
        }

        while self.score >= self.next_city_restore_score {
            if let Some(city) = self.cities.iter_mut().find(|city| !city.alive) {
                city.alive = true;
            }
            self.next_city_restore_score += CITY_RESTORE_STEP;
        }
    }

    fn score_multiplier(&self) -> u32 {
        ((self.wave.saturating_sub(1)) / 2 + 1).min(6)
    }
}

fn make_stars(screen: Vec2) -> Vec<Star> {
    let mut stars = Vec::new();
    for _ in 0..56 {
        stars.push(Star {
            position: vec2(gen_range(0.0, screen.x), gen_range(0.0, screen.y * 0.64)),
            radius: gen_range(0.8, 2.2),
            alpha: gen_range(0.2, 0.9),
        });
    }
    stars
}

fn pick_random_slot(slots: &[TargetSlot]) -> Option<TargetSlot> {
    if slots.is_empty() {
        None
    } else {
        Some(slots[gen_range(0, slots.len() as i32) as usize])
    }
}

fn slot_position(slot: TargetSlot, bases: &[Base; 3], cities: &[City; 6]) -> Vec2 {
    match slot.kind {
        SiteKind::Base => bases[slot.index].position,
        SiteKind::City => cities[slot.index].position,
    }
}

fn nearest_blast(position: Vec2, blasts: &[(Vec2, f32)]) -> Option<(Vec2, f32)> {
    blasts
        .iter()
        .min_by(|a, b| {
            position
                .distance_squared(a.0)
                .partial_cmp(&position.distance_squared(b.0))
                .unwrap_or(core::cmp::Ordering::Equal)
        })
        .copied()
}

fn enemy_score(kind: EnemyKind) -> u32 {
    match kind {
        EnemyKind::Basic | EnemyKind::Splitter { .. } => 25,
        EnemyKind::SmartBomb { .. } => 125,
    }
}
