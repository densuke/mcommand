use macroquad::prelude::*;
use macroquad::rand::gen_range;

const BASE_AMMO: i32 = 10;
const CURSOR_SPEED: f32 = 720.0;
const PLAYER_MAX_RADIUS: f32 = 58.0;
const ENEMY_BLAST_RADIUS: f32 = 24.0;
const CITY_RESTORE_STEP: u32 = 10_000;

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
    Splitter { split_progress: f32 },
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

pub struct Game {
    layout: Layout,
    stars: Vec<Star>,
    cursor: Vec2,
    last_mouse_position: Option<Vec2>,
    bases: [Base; 3],
    cities: [City; 6],
    player_missiles: Vec<PlayerMissile>,
    enemy_missiles: Vec<EnemyMissile>,
    explosions: Vec<Explosion>,
    score: u32,
    next_city_restore_score: u32,
    wave: u32,
    enemies_to_spawn: u32,
    enemies_spawned: u32,
    spawn_timer: f32,
    intermission_timer: f32,
    wave_banner_timer: f32,
    paused: bool,
    game_over: bool,
}

impl Game {
    pub fn new(screen: Vec2) -> Self {
        let layout = Layout::new(screen);
        let cursor = vec2(screen.x * 0.5, screen.y * 0.38);
        let mut game = Self {
            layout,
            stars: make_stars(screen),
            cursor,
            last_mouse_position: None,
            bases: [Base {
                position: vec2(0.0, 0.0),
                ammo: BASE_AMMO,
                alive: true,
            }; 3],
            cities: [City {
                position: vec2(0.0, 0.0),
                alive: true,
            }; 6],
            player_missiles: Vec::new(),
            enemy_missiles: Vec::new(),
            explosions: Vec::new(),
            score: 0,
            next_city_restore_score: CITY_RESTORE_STEP,
            wave: 1,
            enemies_to_spawn: 0,
            enemies_spawned: 0,
            spawn_timer: 0.0,
            intermission_timer: 0.0,
            wave_banner_timer: 1.5,
            paused: false,
            game_over: false,
        };
        game.reset_campaign();
        game
    }

    pub fn update(&mut self, dt: f32) -> bool {
        if is_key_pressed(KeyCode::Q) {
            return true;
        }

        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::P) {
            if !self.game_over {
                self.paused = !self.paused;
            }
        }

        self.update_cursor(dt);
        self.handle_fire_input();

        if self.paused || self.game_over {
            return false;
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
            return false;
        }

        self.update_player_missiles(dt);
        self.update_enemy_missiles(dt);
        self.update_explosions(dt);
        self.handle_explosion_hits();
        self.update_wave_spawning(dt);

        if self.enemies_spawned == self.enemies_to_spawn
            && self.enemy_missiles.is_empty()
            && self.intermission_timer <= 0.0
        {
            self.finish_wave();
        }

        false
    }

    pub fn draw(&self) {
        draw_sky(self.layout);
        draw_ground(self.layout);
        self.draw_stars();
        self.draw_enemy_missiles();
        self.draw_player_missiles();
        self.draw_explosions();
        self.draw_cities();
        self.draw_bases();
        self.draw_cursor();
        self.draw_hud();
        self.draw_overlay();
    }

    fn reset_campaign(&mut self) {
        self.score = 0;
        self.next_city_restore_score = CITY_RESTORE_STEP;
        self.wave = 1;
        self.enemies_spawned = 0;
        self.enemies_to_spawn = 0;
        self.spawn_timer = 0.0;
        self.intermission_timer = 0.0;
        self.wave_banner_timer = 2.0;
        self.paused = false;
        self.game_over = false;
        self.player_missiles.clear();
        self.enemy_missiles.clear();
        self.explosions.clear();

        for (index, base) in self.bases.iter_mut().enumerate() {
            *base = Base {
                position: self.layout.base_positions[index],
                ammo: BASE_AMMO,
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

    fn begin_wave(&mut self) {
        self.enemies_to_spawn = 12 + self.wave * 3;
        self.enemies_spawned = 0;
        self.spawn_timer = 1.1;
        self.wave_banner_timer = 1.75;
        self.player_missiles.clear();
        self.enemy_missiles.clear();
        self.explosions.clear();

        for (index, base) in self.bases.iter_mut().enumerate() {
            *base = Base {
                position: self.layout.base_positions[index],
                ammo: BASE_AMMO,
                alive: true,
            };
        }
    }

    fn finish_wave(&mut self) {
        if self.game_over {
            return;
        }

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
        if self.paused || self.game_over || self.intermission_timer > 0.0 {
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
        let speed = if base_index == 1 { 850.0 } else { 700.0 };
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
                    max_radius: PLAYER_MAX_RADIUS,
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
        self.explosions.extend(detonations);
    }

    fn update_enemy_missiles(&mut self, dt: f32) {
        let mut spawned_children = Vec::new();
        let mut detonations = Vec::new();
        let mut destroyed_sites = Vec::new();
        let available_targets = self.available_target_slots();
        let bases = self.bases;
        let cities = self.cities;

        self.enemy_missiles.retain_mut(|missile| {
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

        self.enemy_missiles.extend(spawned_children);
        self.explosions.extend(detonations);

        for slot in destroyed_sites {
            self.destroy_site(slot);
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
                score_events += 25;
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

        self.explosions.extend(detonations);
        if score_events > 0 {
            self.award_points(score_events);
        }
    }

    fn update_wave_spawning(&mut self, dt: f32) {
        if self.enemies_spawned >= self.enemies_to_spawn {
            return;
        }

        self.spawn_timer -= dt;
        while self.spawn_timer <= 0.0 && self.enemies_spawned < self.enemies_to_spawn {
            self.spawn_enemy_missile();
            self.enemies_spawned += 1;
            self.spawn_timer += self.next_spawn_interval();
        }
    }

    fn spawn_enemy_missile(&mut self) {
        let Some(target_slot) = self.random_target_slot() else {
            self.game_over = true;
            return;
        };

        let start = vec2(
            gen_range(48.0, self.layout.screen.x - 48.0),
            self.layout.horizon_y,
        );
        let target = self.target_position(target_slot);
        let base_speed = 75.0 + self.wave as f32 * 9.0;
        let kind = if self.wave >= 3 && gen_range(0.0, 1.0) < 0.22 {
            EnemyKind::Splitter {
                split_progress: gen_range(0.33, 0.68),
            }
        } else {
            EnemyKind::Basic
        };

        let color = match kind {
            EnemyKind::Basic => color_u8!(255, 94, 80, 255),
            EnemyKind::Splitter { .. } => color_u8!(255, 64, 200, 255),
        };

        self.enemy_missiles.push(EnemyMissile {
            position: start,
            start,
            target,
            target_slot,
            speed: base_speed,
            kind,
            color,
        });
    }

    fn next_spawn_interval(&self) -> f32 {
        (0.95 - self.wave as f32 * 0.03).clamp(0.14, 0.95)
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
            self.game_over = true;
            self.player_missiles.clear();
            self.enemy_missiles.clear();
        }
    }

    fn living_city_count(&self) -> usize {
        self.cities.iter().filter(|city| city.alive).count()
    }

    fn award_points(&mut self, base_points: u32) {
        let points = base_points * self.score_multiplier();
        self.score = self.score.saturating_add(points);

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

    fn draw_enemy_missiles(&self) {
        for missile in &self.enemy_missiles {
            draw_line(
                missile.start.x,
                missile.start.y,
                missile.position.x,
                missile.position.y,
                2.0,
                missile.color,
            );
            draw_circle(missile.position.x, missile.position.y, 3.5, WHITE);
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

    fn draw_hud(&self) {
        let top = 34.0;
        let left = 32.0;
        let right = self.layout.screen.x - 420.0;
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
            "MOVE: ARROWS / MOUSE   FIRE: Z X C   PAUSE: SPACE/P   QUIT: Q",
            left,
            self.layout.screen.y - 18.0,
            TextParams {
                font_size: 20,
                color: color_u8!(190, 198, 206, 255),
                ..Default::default()
            },
        );
    }

    fn draw_overlay(&self) {
        if self.wave_banner_timer > 0.0 {
            let alpha = (self.wave_banner_timer / 1.75).clamp(0.0, 1.0);
            let text = format!("WAVE {}", self.wave);
            draw_centered_text(
                &text,
                self.layout.screen * 0.5 + vec2(0.0, -40.0),
                54,
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
                self.layout.screen * 0.5 + vec2(0.0, -12.0),
                48,
                color_u8!(255, 240, 180, 255),
            );
        }

        if self.game_over {
            draw_rectangle(
                0.0,
                0.0,
                self.layout.screen.x,
                self.layout.screen.y,
                Color::new(0.0, 0.0, 0.0, 0.34),
            );
            draw_centered_text(
                "GAME OVER",
                self.layout.screen * 0.5 + vec2(0.0, -24.0),
                62,
                color_u8!(255, 96, 88, 255),
            );
            draw_centered_text(
                "ALL CITIES LOST  |  PRESS Q TO EXIT",
                self.layout.screen * 0.5 + vec2(0.0, 22.0),
                24,
                color_u8!(240, 228, 220, 255),
            );
        }
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

fn blend(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        1.0,
    )
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
