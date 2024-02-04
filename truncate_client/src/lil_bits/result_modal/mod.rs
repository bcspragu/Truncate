use epaint::{emath::Align2, vec2, Color32, TextureHandle};
use instant::Duration;
use interpolation::Ease;
use truncate_core::{
    game::Game,
    messages::{DailyStats, PlayerMessage},
};

mod graph;
mod msg_mock;

use eframe::egui::{self, Id, Order, Sense};

use crate::{
    app_outer::Backchannel,
    utils::{daily, depot::TruncateDepot, macros::tr_log, text::TextHelper, Lighten, Theme},
};

use self::{graph::DailySplashGraph, msg_mock::ShareMessageMock};

/*

TODOs for the daily splash screen:
- Vertically center contents, or size the modal to the contents better
- Add ability to close the splash screen and look at the game

 */

#[derive(Clone)]
pub struct ResultModalDaily {
    pub stats: DailyStats,
    graph: DailySplashGraph,
    msg_mock: ShareMessageMock,
    streak_length: usize,
    win_rate: f32,
}

#[derive(Clone)]
pub struct ResultModalUnique {
    won: bool,
    msg_mock: ShareMessageMock,
}

#[derive(Clone)]
pub struct ResultModalResigning {
    msg: String,
}

#[derive(Clone)]
pub enum ResultModalVariant {
    Daily(ResultModalDaily),
    Unique(ResultModalUnique),
    Resigning(ResultModalResigning),
}

#[derive(Clone)]
pub struct ResultModalUI {
    pub contents: ResultModalVariant,
}

impl ResultModalUI {
    pub fn new_daily(
        ui: &mut egui::Ui,
        game: &Game,
        depot: &mut TruncateDepot,
        stats: DailyStats,
    ) -> Self {
        let streak_length = stats
            .days
            .values()
            .rev()
            .enumerate()
            .find_map(|(streak_length, day)| {
                if day.attempts.last().map(|a| a.won) == Some(true) {
                    None
                } else {
                    Some(streak_length)
                }
            })
            .unwrap_or_else(|| stats.days.len());

        let win_count = stats
            .days
            .values()
            .filter(|day| day.attempts.last().map(|a| a.won) == Some(true))
            .count();

        let game_count: usize = stats.days.values().map(|day| day.attempts.len()).sum();

        ResultModalUI::seed_animations(ui);

        let graph = DailySplashGraph::new(ui, &stats, depot.timing.current_time);
        let msg_mock = ShareMessageMock::new_daily(game, &depot, &stats);

        Self {
            contents: ResultModalVariant::Daily(ResultModalDaily {
                stats,
                graph,
                msg_mock,
                streak_length,
                win_rate: win_count as f32 / game_count as f32,
            }),
        }
    }

    pub fn new_unique(
        ui: &mut egui::Ui,
        game: &Game,
        depot: &mut TruncateDepot,
        won: bool,
    ) -> Self {
        ResultModalUI::seed_animations(ui);

        Self {
            contents: ResultModalVariant::Unique(ResultModalUnique {
                won,
                msg_mock: ShareMessageMock::new_unique(game, &depot),
            }),
        }
    }
    pub fn new_resigning(ui: &mut egui::Ui, msg: String) -> Self {
        ResultModalUI::seed_animations(ui);

        Self {
            contents: ResultModalVariant::Resigning(ResultModalResigning { msg }),
        }
    }
}

#[derive(Hash)]
enum Anim {
    Background,
    ModalPos,
    Items,
    Viz,
}

impl Into<Id> for Anim {
    fn into(self) -> Id {
        Id::new("splash").with(self)
    }
}

impl ResultModalUI {
    fn seed_animations(ui: &mut egui::Ui) {
        ResultModalUI::anim(ui, Anim::Background, 0.0, 0.0);
        ResultModalUI::anim(ui, Anim::ModalPos, 0.0, 0.0);
        ResultModalUI::anim(ui, Anim::Items, 0.0, 0.0);
        ResultModalUI::anim(ui, Anim::Viz, 0.0, 0.0);
    }

    // Animates to a given value once (can't be retriggered), applying an easing function
    fn anim(ui: &mut egui::Ui, val: Anim, to: f32, duration: f32) -> f32 {
        let linear = ui.ctx().animate_value_with_time(
            val.into(),
            if to > 0.0 { 1.0 } else { 0.0 },
            duration,
        );
        let eased = linear.calc(interpolation::EaseFunction::QuadraticOut);
        eased * to
    }
}

pub enum ResultModalAction {
    TryAgain,
    NewPuzzle,
    Dismiss,
    Resign,
}

impl ResultModalUI {
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        map_texture: &TextureHandle,
        backchannel: Option<&Backchannel>,
    ) -> Option<ResultModalAction> {
        let mut msg = None;

        let area = egui::Area::new(egui::Id::new("daily_splash_layer"))
            .movable(false)
            .order(Order::Foreground)
            .anchor(Align2::LEFT_TOP, vec2(0.0, 0.0));

        let ideal_modal_width = 370.0;
        let ideal_modal_height = match self.contents {
            ResultModalVariant::Daily(_) => 620.0,
            ResultModalVariant::Unique(_) => 570.0,
            ResultModalVariant::Resigning(_) => 300.0,
        };

        area.show(ui.ctx(), |ui| {
            let screen_dimension = ui.max_rect();

            // Capture events on our overlay to stop them falling through to the game
            ui.allocate_rect(screen_dimension, Sense::click());

            let bg_alpha = ResultModalUI::anim(ui, Anim::Background, 0.5, 1.5);
            let bg = Color32::BLACK.gamma_multiply(bg_alpha);

            ui.painter().clone().rect_filled(screen_dimension, 10.0, bg);

            // Wait for the background overlay to start animating before showing the modal
            if bg_alpha < 0.3 {
                return;
            }

            let mut x_difference = (screen_dimension.width() - ideal_modal_width) / 2.0;
            let mut y_difference = (screen_dimension.height() - ideal_modal_height) / 2.0;

            if x_difference < 10.0 {
                x_difference = 10.0;
            }
            if y_difference < 10.0 {
                y_difference = 10.0;
            }

            let mut modal_dimension = screen_dimension.shrink2(vec2(x_difference, y_difference));

            let modal_pos = ResultModalUI::anim(ui, Anim::ModalPos, 1.0, 0.5);
            let bg = Color32::BLACK.gamma_multiply(modal_pos); // Fade in the modal background
            let offset = (1.0 - modal_pos) * 40.0;
            modal_dimension = modal_dimension.translate(vec2(0.0, offset)); // Animate the modal in vertically

            ui.painter().rect_filled(modal_dimension, 0.0, bg);

            // Wait for the modal position to be close before showing the contents
            if modal_pos < 0.7 {
                return;
            }

            let modal_inner_dimension = modal_dimension.shrink(30.0);
            ui.allocate_ui_at_rect(modal_inner_dimension, |mut ui| {
                // TODO: Add close button (reference game sidebar on mobile)

                let modal_items = ResultModalUI::anim(ui, Anim::Items, 1.0, 0.75);
                let offset = (1.0 - modal_items) * 50.0;
                ui.add_space(offset); // Animate the main text upward

                ui.spacing_mut().item_spacing = vec2(0.0, 10.0);

                ui.add_space(10.0);

                match &mut self.contents {
                    ResultModalVariant::Daily(daily) => {
                        let streak_string = format!("{} day streak", daily.streak_length);
                        let streak_text = TextHelper::heavy(&streak_string, 14.0, None, &mut ui);
                        streak_text.paint(Color32::WHITE, ui, true);

                        ui.add_space(4.0);

                        let wr_string = format!("{}% win rate", (daily.win_rate * 100.0) as usize);
                        let wr_text = TextHelper::heavy(&wr_string, 12.0, None, &mut ui);
                        wr_text.paint(Color32::WHITE, ui, true);
                    }
                    ResultModalVariant::Unique(u) => {
                        if u.won {
                            let summary_string = "Great job!".to_string();
                            let summary_text =
                                TextHelper::heavy(&summary_string, 14.0, None, &mut ui);
                            summary_text.paint(Color32::WHITE, ui, true);
                        } else {
                            let summary_string = "No worries,".to_string();
                            let summary_text =
                                TextHelper::heavy(&summary_string, 14.0, None, &mut ui);
                            summary_text.paint(Color32::WHITE, ui, true);
                            let summary_string = "have another go!".to_string();
                            let summary_text =
                                TextHelper::heavy(&summary_string, 14.0, None, &mut ui);
                            summary_text.paint(Color32::WHITE, ui, true);
                        }
                    }
                    ResultModalVariant::Resigning(r) => {
                        let summary_text = TextHelper::heavy(&r.msg, 14.0, None, &mut ui);
                        summary_text.paint(Color32::WHITE, ui, true);
                    }
                }

                // Wait for the main text to move out of the way before showing details
                if modal_items < 0.9 {
                    return;
                }

                let modal_remainder = ui.available_rect_before_wrap();

                if let ResultModalVariant::Daily(daily) = &mut self.contents {
                    ui.add_space(16.0);
                    daily.graph.render(ui);
                }

                ui.add_space(20.0);

                match &mut self.contents {
                    ResultModalVariant::Daily(daily) => {
                        let won_today = daily
                            .stats
                            .days
                            .values()
                            .last()
                            .cloned()
                            .unwrap_or_default()
                            .attempts
                            .last()
                            .is_some_and(|a| a.won);

                        let won_yesterday = daily
                            .stats
                            .days
                            .values()
                            .nth_back(1)
                            .cloned()
                            .unwrap_or_default()
                            .attempts
                            .last()
                            .is_some_and(|a| a.won);

                        if won_today {
                            daily.msg_mock.render(ui, theme, map_texture, backchannel);
                        } else {
                            ui.add_space(20.0);

                            let mut textrow = |string: String| {
                                let row = TextHelper::heavy(
                                    &string,
                                    14.0,
                                    Some(ui.available_width()),
                                    &mut ui,
                                );
                                row.paint(Color32::WHITE, ui, true);
                            };

                            if won_yesterday {
                                textrow("Try again".into());
                                textrow("to maintain".into());
                                textrow("your streak!".into());
                            } else {
                                textrow("No worries,".into());
                                textrow("have another go!".into());
                            }

                            ui.add_space(16.0);

                            let text = TextHelper::heavy("TRY AGAIN", 12.0, None, ui);
                            let try_again_button = text.centered_button(
                                theme.button_primary,
                                theme.text,
                                map_texture,
                                ui,
                            );
                            if try_again_button.clicked() {
                                msg = Some(ResultModalAction::TryAgain);
                            }
                        }
                    }
                    ResultModalVariant::Unique(unique) => {
                        unique.msg_mock.render(ui, theme, map_texture, backchannel);

                        ui.add_space(20.0);
                        let text = TextHelper::heavy("TRY AGAIN", 12.0, None, ui);
                        let try_again_button =
                            text.centered_button(theme.button_primary, theme.text, map_texture, ui);
                        if try_again_button.clicked() {
                            msg = Some(ResultModalAction::TryAgain);
                        }

                        ui.add_space(10.0);
                        let text = TextHelper::heavy("NEW PUZZLE", 12.0, None, ui);
                        let new_puzzle_button =
                            text.centered_button(theme.button_primary, theme.text, map_texture, ui);
                        if new_puzzle_button.clicked() {
                            msg = Some(ResultModalAction::NewPuzzle);
                        }
                    }
                    ResultModalVariant::Resigning(_r) => {
                        ui.add_space(20.0);
                        let text = TextHelper::heavy("RESIGN", 12.0, None, ui);
                        let try_again_button =
                            text.centered_button(theme.button_primary, theme.text, map_texture, ui);
                        if try_again_button.clicked() {
                            msg = Some(ResultModalAction::Resign);
                        }

                        ui.add_space(10.0);
                        let text = TextHelper::heavy("CONTINUE PLAYING", 12.0, None, ui);
                        let new_puzzle_button = text.centered_button(
                            theme.water.lighten().lighten(),
                            theme.text,
                            map_texture,
                            ui,
                        );
                        if new_puzzle_button.clicked() {
                            msg = Some(ResultModalAction::Dismiss);
                        }
                    }
                };

                // Paint over everything below the heading stats to fade them in from black
                let fade_in_animation = ResultModalUI::anim(ui, Anim::Viz, 1.0, 0.6);
                if fade_in_animation < 1.0 {
                    ui.painter().rect_filled(
                        modal_remainder,
                        0.0,
                        Color32::BLACK.gamma_multiply(1.0 - fade_in_animation),
                    )
                }
            });
        });

        msg
    }
}