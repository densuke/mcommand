use super::*;

impl Game {
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

fn blend(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        1.0,
    )
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
