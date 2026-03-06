use core::f32::consts::PI;

use macroquad::audio::{
    PlaySoundParams, Sound, load_sound_from_bytes, play_sound, play_sound_once,
};
use macroquad::prelude::*;
use macroquad::rand::gen_range;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

const CURSOR_SPEED: f32 = 720.0;
const ENEMY_BLAST_RADIUS: f32 = 24.0;
const CITY_RESTORE_STEP: u32 = 10_000;

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

#[derive(Clone, Copy)]
struct SaveData {
    config: GameConfig,
    high_score: u32,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            config: GameConfig::default(),
            high_score: 0,
        }
    }
}

impl SaveData {
    fn encode(self) -> String {
        format!("{}\nhigh_score={}", self.config.encode(), self.high_score)
    }

    fn decode(raw: &str) -> Self {
        let mut save = Self::default();
        if let Some(config) = GameConfig::decode(raw) {
            save.config = config;
        }
        for line in raw.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key.trim() == "high_score" {
                if let Ok(score) = value.trim().parse() {
                    save.high_score = score;
                }
            }
        }
        save
    }
}

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

#[derive(Default)]
struct AudioBank {
    music: Option<Sound>,
    fire: Option<Sound>,
    explosion: Option<Sound>,
    smart_bomb: Option<Sound>,
    ui_move: Option<Sound>,
    start: Option<Sound>,
    game_over: Option<Sound>,
    music_started: bool,
}

impl AudioBank {
    async fn load() -> Self {
        Self {
            music: load_sound_from_bytes(&make_music_wav()).await.ok(),
            fire: load_sound_from_bytes(&make_tone_wav(430.0, 0.08, 0.25, Waveform::Pulse(0.35)))
                .await
                .ok(),
            explosion: load_sound_from_bytes(&make_noise_wav(0.22, 0.40))
                .await
                .ok(),
            smart_bomb: load_sound_from_bytes(&make_smart_bomb_wav()).await.ok(),
            ui_move: load_sound_from_bytes(&make_tone_wav(780.0, 0.05, 0.12, Waveform::Sine))
                .await
                .ok(),
            start: load_sound_from_bytes(&make_start_wav()).await.ok(),
            game_over: load_sound_from_bytes(&make_game_over_wav()).await.ok(),
            music_started: false,
        }
    }

    fn ensure_music(&mut self) {
        if self.music_started {
            return;
        }
        if let Some(sound) = &self.music {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: true,
                    volume: 0.28,
                },
            );
            self.music_started = true;
        }
    }

    fn play_fire(&self) {
        if let Some(sound) = &self.fire {
            play_sound_once(sound);
        }
    }

    fn play_explosion(&self) {
        if let Some(sound) = &self.explosion {
            play_sound_once(sound);
        }
    }

    fn play_smart_bomb(&self) {
        if let Some(sound) = &self.smart_bomb {
            play_sound_once(sound);
        }
    }

    fn play_ui_move(&self) {
        if let Some(sound) = &self.ui_move {
            play_sound_once(sound);
        }
    }

    fn play_start(&self) {
        if let Some(sound) = &self.start {
            play_sound_once(sound);
        }
    }

    fn play_game_over(&self) {
        if let Some(sound) = &self.game_over {
            play_sound_once(sound);
        }
    }
}

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

    pub fn draw(&self) {
        draw_sky(self.layout);
        draw_ground(self.layout);
        self.draw_stars();

        if matches!(self.mode, ScreenState::Playing | ScreenState::GameOver) {
            self.draw_satellites();
            self.draw_bombers();
            self.draw_enemy_missiles();
            self.draw_player_missiles();
            self.draw_explosions();
        }

        self.draw_cities();
        self.draw_bases();

        match self.mode {
            ScreenState::Title => self.draw_title_screen(),
            ScreenState::Settings => self.draw_settings_screen(),
            ScreenState::Playing => {
                self.draw_cursor();
                self.draw_hud();
                self.draw_play_overlay();
            }
            ScreenState::GameOver => {
                self.draw_hud();
                self.draw_game_over_screen();
            }
        }
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

    fn draw_title_screen(&self) {
        draw_centered_text(
            "MISSILE COMMAND",
            self.layout.screen * 0.5 + vec2(0.0, -180.0),
            74,
            color_u8!(248, 210, 130, 255),
        );
        draw_centered_text(
            "Rust / macroquad prototype",
            self.layout.screen * 0.5 + vec2(0.0, -132.0),
            24,
            color_u8!(200, 220, 236, 255),
        );

        draw_panel(Rect::new(
            self.layout.screen.x * 0.29,
            self.layout.screen.y * 0.34,
            self.layout.screen.x * 0.42,
            self.layout.screen.y * 0.28,
        ));

        draw_centered_text(
            "ENTER / SPACE  START DEFENSE",
            self.layout.screen * 0.5 + vec2(0.0, -20.0),
            30,
            color_u8!(142, 255, 180, 255),
        );
        draw_centered_text(
            title_action_text(),
            self.layout.screen * 0.5 + vec2(0.0, 18.0),
            24,
            color_u8!(236, 242, 248, 255),
        );
        draw_centered_text(
            &format!(
                "AMMO {}   BLAST {:.0}px   DIFFICULTY {}",
                self.config.ammo_per_base,
                self.config.blast_radius,
                self.config.difficulty.label()
            ),
            self.layout.screen * 0.5 + vec2(0.0, 72.0),
            24,
            color_u8!(164, 210, 255, 255),
        );
        draw_centered_text(
            self.config.difficulty.description(),
            self.layout.screen * 0.5 + vec2(0.0, 108.0),
            20,
            color_u8!(196, 206, 214, 255),
        );
        draw_centered_text(
            &format!("HIGH SCORE {:06}   F FULLSCREEN", self.high_score),
            self.layout.screen * 0.5 + vec2(0.0, 146.0),
            20,
            color_u8!(196, 204, 212, 255),
        );
    }

    fn draw_settings_screen(&self) {
        draw_centered_text(
            "SETTINGS",
            self.layout.screen * 0.5 + vec2(0.0, -200.0),
            64,
            color_u8!(248, 210, 130, 255),
        );

        let panel = Rect::new(
            self.layout.screen.x * 0.24,
            self.layout.screen.y * 0.26,
            self.layout.screen.x * 0.52,
            self.layout.screen.y * 0.44,
        );
        draw_panel(panel);

        let entries = [
            format!("BASE MISSILES      {:>2}", self.config.ammo_per_base),
            format!("BLAST RADIUS       {:>2.0}px", self.config.blast_radius),
            format!("DIFFICULTY         {}", self.config.difficulty.label()),
            "BACK".to_owned(),
        ];

        for (index, entry) in entries.iter().enumerate() {
            let y = panel.y + 74.0 + index as f32 * 58.0;
            let selected = index == self.settings_cursor;
            if selected {
                draw_rectangle(
                    panel.x + 34.0,
                    y - 28.0,
                    panel.w - 68.0,
                    40.0,
                    Color::new(0.12, 0.22, 0.30, 0.85),
                );
            }
            draw_text_ex(
                entry,
                panel.x + 62.0,
                y,
                TextParams {
                    font_size: 28,
                    color: if selected {
                        color_u8!(142, 255, 180, 255)
                    } else {
                        color_u8!(232, 238, 246, 255)
                    },
                    ..Default::default()
                },
            );
        }

        draw_text_ex(
            self.config.difficulty.description(),
            panel.x + 62.0,
            panel.y + panel.h - 82.0,
            TextParams {
                font_size: 22,
                color: color_u8!(166, 220, 255, 255),
                ..Default::default()
            },
        );
        draw_text_ex(
            "UP/DOWN: select   LEFT/RIGHT: adjust   R: defaults   ESC/ENTER: back",
            panel.x + 62.0,
            panel.y + panel.h - 34.0,
            TextParams {
                font_size: 20,
                color: color_u8!(196, 204, 212, 255),
                ..Default::default()
            },
        );
    }

    fn draw_hud(&self) {
        let top = 34.0;
        let left = 32.0;
        let right = self.layout.screen.x - 430.0;
        let hud_color = color_u8!(232, 238, 245, 255);
        let accent = color_u8!(142, 255, 180, 255);

        draw_text_ex(
            &format!("SCORE {:06}", self.score),
            left,
            top,
            TextParams {
                font_size: 34,
                color: hud_color,
                ..Default::default()
            },
        );
        draw_text_ex(
            &format!("WAVE {}  x{}", self.wave, self.score_multiplier()),
            right,
            top,
            TextParams {
                font_size: 30,
                color: accent,
                ..Default::default()
            },
        );
        draw_text_ex(
            &format!("CITIES {}", self.living_city_count()),
            left,
            top + 32.0,
            TextParams {
                font_size: 22,
                color: color_u8!(166, 220, 255, 255),
                ..Default::default()
            },
        );
        draw_text_ex(
            &format!("HIGH {:06}", self.high_score),
            left,
            top + 58.0,
            TextParams {
                font_size: 20,
                color: color_u8!(236, 216, 142, 255),
                ..Default::default()
            },
        );
        draw_text_ex(
            &format!(
                "AMMO {}  BLAST {:.0}px  {}",
                self.config.ammo_per_base,
                self.config.blast_radius,
                self.config.difficulty.label()
            ),
            right,
            top + 30.0,
            TextParams {
                font_size: 20,
                color: color_u8!(220, 220, 196, 255),
                ..Default::default()
            },
        );
        draw_text_ex(
            footer_action_text(),
            left,
            self.layout.screen.y - 18.0,
            TextParams {
                font_size: 20,
                color: color_u8!(190, 198, 206, 255),
                ..Default::default()
            },
        );
    }

    fn draw_play_overlay(&self) {
        if self.wave_banner_timer > 0.0 {
            let alpha = (self.wave_banner_timer / 1.75).clamp(0.0, 1.0);
            draw_centered_text(
                &format!("WAVE {}", self.wave),
                self.layout.screen * 0.5 + vec2(0.0, -46.0),
                56,
                Color::new(0.96, 0.98, 1.0, alpha),
            );
        }

        if self.intermission_timer > 0.0 {
            draw_centered_text(
                "SECTOR SECURED",
                self.layout.screen * 0.5 + vec2(0.0, -8.0),
                36,
                color_u8!(142, 255, 180, 255),
            );
            draw_centered_text(
                "MISSILES AND CITIES TALLIED",
                self.layout.screen * 0.5 + vec2(0.0, 26.0),
                24,
                color_u8!(220, 228, 236, 255),
            );
        }

        if self.paused {
            draw_rectangle(
                0.0,
                0.0,
                self.layout.screen.x,
                self.layout.screen.y,
                Color::new(0.0, 0.0, 0.0, 0.28),
            );
            draw_centered_text(
                "PAUSED",
                self.layout.screen * 0.5 + vec2(0.0, -14.0),
                48,
                color_u8!(255, 240, 180, 255),
            );
        }
    }

    fn draw_game_over_screen(&self) {
        draw_rectangle(
            0.0,
            0.0,
            self.layout.screen.x,
            self.layout.screen.y,
            Color::new(0.0, 0.0, 0.0, 0.34),
        );
        draw_centered_text(
            "GAME OVER",
            self.layout.screen * 0.5 + vec2(0.0, -48.0),
            62,
            color_u8!(255, 96, 88, 255),
        );
        draw_centered_text(
            &format!("FINAL SCORE {:06}   WAVE {}", self.score, self.wave),
            self.layout.screen * 0.5 + vec2(0.0, 2.0),
            28,
            color_u8!(240, 228, 220, 255),
        );
        draw_centered_text(
            game_over_action_text(),
            self.layout.screen * 0.5 + vec2(0.0, 42.0),
            22,
            color_u8!(220, 228, 236, 255),
        );
    }

    fn draw_stars(&self) {
        for star in &self.stars {
            draw_circle(
                star.position.x,
                star.position.y,
                star.radius,
                Color::new(0.95, 0.98, 1.0, star.alpha),
            );
        }
    }

    fn draw_bombers(&self) {
        for bomber in &self.bombers {
            let body = color_u8!(255, 198, 104, 255);
            let wing = color_u8!(255, 136, 72, 255);
            draw_rectangle(
                bomber.position.x - 22.0,
                bomber.position.y - 5.0,
                44.0,
                10.0,
                body,
            );
            draw_triangle(
                vec2(bomber.position.x - 28.0, bomber.position.y),
                vec2(bomber.position.x - 6.0, bomber.position.y - 10.0),
                vec2(bomber.position.x - 6.0, bomber.position.y + 10.0),
                wing,
            );
            draw_triangle(
                vec2(bomber.position.x + 28.0, bomber.position.y),
                vec2(bomber.position.x + 6.0, bomber.position.y - 10.0),
                vec2(bomber.position.x + 6.0, bomber.position.y + 10.0),
                wing,
            );
        }
    }

    fn draw_satellites(&self) {
        for satellite in &self.satellites {
            let glow = color_u8!(140, 230, 255, 120);
            let hull = color_u8!(224, 246, 255, 255);
            draw_circle(satellite.position.x, satellite.position.y, 11.0, glow);
            draw_rectangle(
                satellite.position.x - 12.0,
                satellite.position.y - 2.0,
                24.0,
                4.0,
                hull,
            );
            draw_line(
                satellite.position.x,
                satellite.position.y - 8.0,
                satellite.position.x,
                satellite.position.y + 8.0,
                2.0,
                hull,
            );
        }
    }

    fn draw_enemy_missiles(&self) {
        for missile in &self.enemy_missiles {
            draw_line(
                missile.start.x,
                missile.start.y,
                missile.position.x,
                missile.position.y,
                if matches!(missile.kind, EnemyKind::SmartBomb { .. }) {
                    3.2
                } else {
                    2.0
                },
                missile.color,
            );
            draw_circle(missile.position.x, missile.position.y, 3.6, WHITE);
            if matches!(missile.kind, EnemyKind::SmartBomb { .. }) {
                draw_circle_lines(
                    missile.position.x,
                    missile.position.y,
                    6.2,
                    1.2,
                    missile.color,
                );
            }
        }
    }

    fn draw_player_missiles(&self) {
        for missile in &self.player_missiles {
            draw_line(
                missile.position.x,
                missile.position.y,
                missile.target.x,
                missile.target.y,
                1.4,
                Color::new(missile.color.r, missile.color.g, missile.color.b, 0.18),
            );
            draw_circle(missile.position.x, missile.position.y, 3.0, missile.color);
        }
    }

    fn draw_explosions(&self) {
        for explosion in &self.explosions {
            let (core, halo) = match explosion.owner {
                ExplosionOwner::Player => {
                    (color_u8!(255, 226, 136, 220), color_u8!(255, 156, 64, 90))
                }
                ExplosionOwner::Enemy => (color_u8!(255, 110, 88, 180), color_u8!(255, 60, 60, 70)),
            };
            draw_circle(
                explosion.position.x,
                explosion.position.y,
                explosion.radius,
                halo,
            );
            draw_circle(
                explosion.position.x,
                explosion.position.y,
                explosion.radius * 0.42,
                core,
            );
        }
    }

    fn draw_cities(&self) {
        for city in &self.cities {
            let width = self.layout.screen.x * 0.038;
            let height = self.layout.screen.y * 0.04;
            let left = city.position.x - width * 0.5;
            let top = city.position.y - height;
            let color = if city.alive {
                color_u8!(110, 210, 255, 255)
            } else {
                color_u8!(70, 46, 52, 255)
            };
            draw_rectangle(left, top + height * 0.28, width, height * 0.72, color);
            draw_rectangle(left + width * 0.15, top, width * 0.22, height * 0.36, color);
            draw_rectangle(
                left + width * 0.48,
                top + height * 0.08,
                width * 0.16,
                height * 0.28,
                color,
            );
            draw_rectangle(
                left + width * 0.72,
                top + height * 0.16,
                width * 0.14,
                height * 0.20,
                color,
            );
        }
    }

    fn draw_bases(&self) {
        for (index, base) in self.bases.iter().enumerate() {
            let width = self.layout.screen.x * 0.065;
            let height = self.layout.screen.y * 0.03;
            let left = base.position.x - width * 0.5;
            let top = base.position.y - height;
            let color = if base.alive {
                match index {
                    0 => color_u8!(64, 200, 255, 255),
                    1 => color_u8!(120, 255, 210, 255),
                    _ => color_u8!(255, 220, 96, 255),
                }
            } else {
                color_u8!(72, 44, 40, 255)
            };

            draw_triangle(
                vec2(left, base.position.y),
                vec2(base.position.x, top),
                vec2(left + width, base.position.y),
                color,
            );

            let ammo_text = format!("{}", base.ammo.max(0));
            let label = ["Z", "X", "C"][index];
            draw_text_ex(
                &format!("{} {}", label, ammo_text),
                left,
                base.position.y + 24.0,
                TextParams {
                    font_size: 20,
                    color: color_u8!(225, 230, 236, 255),
                    ..Default::default()
                },
            );
        }
    }

    fn draw_cursor(&self) {
        let color = color_u8!(142, 255, 180, 255);
        let x = self.cursor.x;
        let y = self.cursor.y;
        draw_circle_lines(x, y, 16.0, 1.5, color);
        draw_line(x - 20.0, y, x - 5.0, y, 1.8, color);
        draw_line(x + 5.0, y, x + 20.0, y, 1.8, color);
        draw_line(x, y - 20.0, x, y - 5.0, 1.8, color);
        draw_line(x, y + 5.0, x, y + 20.0, 1.8, color);
    }
}

fn draw_sky(layout: Layout) {
    let bands = 12;
    let top = color_u8!(6, 10, 26, 255);
    let bottom = color_u8!(18, 34, 66, 255);
    for index in 0..bands {
        let t0 = index as f32 / bands as f32;
        let t1 = (index + 1) as f32 / bands as f32;
        let color = blend(top, bottom, t0);
        draw_rectangle(
            0.0,
            layout.screen.y * t0,
            layout.screen.x,
            layout.screen.y * (t1 - t0) + 1.0,
            color,
        );
    }
}

fn draw_ground(layout: Layout) {
    draw_rectangle(
        0.0,
        layout.ground_y,
        layout.screen.x,
        layout.screen.y - layout.ground_y,
        color_u8!(28, 34, 26, 255),
    );
    draw_line(
        0.0,
        layout.ground_y,
        layout.screen.x,
        layout.ground_y,
        2.0,
        color_u8!(160, 188, 124, 255),
    );
}

fn draw_panel(rect: Rect) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.03, 0.06, 0.12, 0.78),
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        2.0,
        color_u8!(142, 255, 180, 180),
    );
}

fn draw_centered_text(text: &str, center: Vec2, font_size: u16, color: Color) {
    let dims = measure_text(text, None, font_size, 1.0);
    draw_text_ex(
        text,
        center.x - dims.width * 0.5,
        center.y + dims.height * 0.5,
        TextParams {
            font_size,
            color,
            ..Default::default()
        },
    );
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

fn blend(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        1.0,
    )
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

fn title_action_text() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    {
        "O / S  SETTINGS    Q  BACK    F  FULLSCREEN"
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "O / S  SETTINGS    Q  QUIT    F  FULLSCREEN"
    }
}

fn footer_action_text() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    {
        "MOVE: ARROWS / MOUSE   FIRE: Z X C   PAUSE: SPACE/P   Q: TITLE/BACK   F: FULLSCREEN"
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "MOVE: ARROWS / MOUSE   FIRE: Z X C   PAUSE: SPACE/P   Q: QUIT   F: FULLSCREEN"
    }
}

fn game_over_action_text() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    {
        "ENTER/R RESTART   T TITLE   O/S SETTINGS   Q TITLE   F FULLSCREEN"
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "ENTER/R RESTART   T TITLE   O/S SETTINGS   Q QUIT   F FULLSCREEN"
    }
}

fn load_save_data() -> SaveData {
    load_save_blob()
        .map(|raw| SaveData::decode(&raw))
        .unwrap_or_default()
}

fn store_save_data(save: SaveData) {
    store_save_blob(&save.encode());
}

#[cfg(target_arch = "wasm32")]
fn load_save_blob() -> Option<String> {
    let len = unsafe { mcommand_storage_get_len() };
    if len <= 0 {
        return None;
    }

    let mut buffer = vec![0u8; len as usize];
    unsafe {
        mcommand_storage_get(buffer.as_mut_ptr(), len as u32);
    }
    String::from_utf8(buffer).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn load_save_blob() -> Option<String> {
    std::fs::read_to_string(native_save_path()).ok()
}

#[cfg(target_arch = "wasm32")]
fn store_save_blob(raw: &str) {
    unsafe {
        mcommand_storage_set(raw.as_ptr(), raw.len() as u32);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn store_save_blob(raw: &str) {
    let _ = std::fs::write(native_save_path(), raw);
}

#[cfg(not(target_arch = "wasm32"))]
fn native_save_path() -> PathBuf {
    let mut path = std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    path.push(".mcommand-save");
    path
}

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn mcommand_storage_get_len() -> i32;
    fn mcommand_storage_get(ptr: *mut u8, len: u32);
    fn mcommand_storage_set(ptr: *const u8, len: u32);
}

enum Waveform {
    Sine,
    Pulse(f32),
}

fn make_tone_wav(freq: f32, seconds: f32, volume: f32, waveform: Waveform) -> Vec<u8> {
    let sample_rate = 22_050;
    let total = (sample_rate as f32 * seconds) as usize;
    let mut samples = Vec::with_capacity(total);

    for index in 0..total {
        let t = index as f32 / sample_rate as f32;
        let env = (1.0 - index as f32 / total as f32).powf(1.4);
        let signal = match waveform {
            Waveform::Sine => (2.0 * PI * freq * t).sin(),
            Waveform::Pulse(duty) => {
                let phase = (freq * t).fract();
                if phase < duty { 1.0 } else { -1.0 }
            }
        };
        samples.push(signal * env * volume);
    }

    make_pcm_wav(sample_rate, &samples)
}

fn make_noise_wav(seconds: f32, volume: f32) -> Vec<u8> {
    let sample_rate = 22_050;
    let total = (sample_rate as f32 * seconds) as usize;
    let mut seed = 0x1234_5678u32;
    let mut samples = Vec::with_capacity(total);

    for index in 0..total {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = ((seed >> 8) as f32 / (u32::MAX >> 8) as f32) * 2.0 - 1.0;
        let env = (1.0 - index as f32 / total as f32).powf(2.2);
        samples.push(noise * env * volume);
    }

    make_pcm_wav(sample_rate, &samples)
}

fn make_start_wav() -> Vec<u8> {
    let sample_rate = 22_050;
    let notes = [(220.0, 0.07), (330.0, 0.07), (440.0, 0.10), (660.0, 0.14)];
    let mut samples = Vec::new();
    for (freq, seconds) in notes {
        samples.extend(make_tone_samples(
            freq,
            seconds,
            0.18,
            Waveform::Pulse(0.4),
            sample_rate,
        ));
    }
    make_pcm_wav(sample_rate, &samples)
}

fn make_game_over_wav() -> Vec<u8> {
    let sample_rate = 22_050;
    let notes = [(330.0, 0.08), (248.0, 0.10), (196.0, 0.14), (147.0, 0.18)];
    let mut samples = Vec::new();
    for (freq, seconds) in notes {
        samples.extend(make_tone_samples(
            freq,
            seconds,
            0.17,
            Waveform::Pulse(0.55),
            sample_rate,
        ));
    }
    make_pcm_wav(sample_rate, &samples)
}

fn make_smart_bomb_wav() -> Vec<u8> {
    let sample_rate = 22_050;
    let total = (sample_rate as f32 * 0.22) as usize;
    let mut samples = Vec::with_capacity(total);
    for index in 0..total {
        let t = index as f32 / sample_rate as f32;
        let freq = 300.0 + 900.0 * (index as f32 / total as f32);
        let env = (1.0 - index as f32 / total as f32).powf(1.1);
        let value = (2.0 * PI * freq * t).sin();
        samples.push(value * env * 0.18);
    }
    make_pcm_wav(sample_rate, &samples)
}

fn make_music_wav() -> Vec<u8> {
    let sample_rate = 22_050;
    let step_seconds = 0.18;
    let lead = [220.0, 330.0, 440.0, 330.0, 246.0, 369.0, 493.0, 369.0];
    let bass = [110.0, 110.0, 123.0, 123.0, 98.0, 98.0, 82.0, 82.0];
    let total = (sample_rate as f32 * step_seconds * lead.len() as f32) as usize;
    let mut samples = Vec::with_capacity(total);

    for (step, lead_freq) in lead.iter().enumerate() {
        let bass_freq = bass[step];
        let step_samples = (sample_rate as f32 * step_seconds) as usize;
        for sample in 0..step_samples {
            let t = sample as f32 / sample_rate as f32;
            let env = (1.0 - sample as f32 / step_samples as f32).powf(0.7);
            let lead_value = if (lead_freq * t).fract() < 0.35 {
                1.0
            } else {
                -1.0
            };
            let bass_value = (2.0 * PI * bass_freq * t).sin();
            samples.push((lead_value * 0.10 + bass_value * 0.06) * env);
        }
    }

    make_pcm_wav(sample_rate, &samples)
}

fn make_tone_samples(
    freq: f32,
    seconds: f32,
    volume: f32,
    waveform: Waveform,
    sample_rate: u32,
) -> Vec<f32> {
    let total = (sample_rate as f32 * seconds) as usize;
    let mut samples = Vec::with_capacity(total);
    for index in 0..total {
        let t = index as f32 / sample_rate as f32;
        let env = (1.0 - index as f32 / total as f32).powf(1.3);
        let signal = match waveform {
            Waveform::Sine => (2.0 * PI * freq * t).sin(),
            Waveform::Pulse(duty) => {
                let phase = (freq * t).fract();
                if phase < duty { 1.0 } else { -1.0 }
            }
        };
        samples.push(signal * env * volume);
    }
    samples
}

fn make_pcm_wav(sample_rate: u32, samples: &[f32]) -> Vec<u8> {
    let byte_rate = sample_rate * 2;
    let data_size = (samples.len() * 2) as u32;
    let riff_size = 36 + data_size;
    let mut bytes = Vec::with_capacity(44 + samples.len() * 2);

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());

    for sample in samples {
        let clamped = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        bytes.extend_from_slice(&clamped.to_le_bytes());
    }

    bytes
}
