use std::sync::Arc;

use eframe::{
    egui::{self, CursorIcon, FontId, Id, Layout, RichText, Sense},
    emath::Align,
};
use epaint::{hex_color, pos2, vec2, Color32, Galley, Pos2, Rect, Stroke, Vec2};
use truncate_core::reporting::{BattleReport, BattleWord};

use crate::{
    regions::active_game::GameCtx,
    utils::{glyph_meaure::GlyphMeasure, Darken},
};

pub struct BattleUI<'a> {
    battle: &'a BattleReport,
}

impl<'a> BattleUI<'a> {
    pub fn new(battle: &'a BattleReport) -> Self {
        Self { battle }
    }
}

fn get_galleys<'a>(
    battle_words: &'a Vec<BattleWord>,
    ctx: &GameCtx,
    ui: &mut egui::Ui,
) -> Vec<Arc<Galley>> {
    let dot = ui.painter().layout_no_wrap(
        "•".into(),
        FontId::new(
            ctx.theme.letter_size * 0.75,
            egui::FontFamily::Name("Truncate-Heavy".into()),
        ),
        ctx.theme.outlines.darken(),
    );

    let mut words: Vec<_> = battle_words
        .iter()
        .flat_map(|w| {
            [
                ui.painter().layout_no_wrap(
                    w.word.clone(),
                    FontId::new(
                        ctx.theme.letter_size * 0.75,
                        egui::FontFamily::Name("Truncate-Heavy".into()),
                    ),
                    match w.valid {
                        Some(true) => ctx.theme.addition.darken(),
                        Some(false) => ctx.theme.defeated.darken(),
                        None => ctx.theme.outlines.darken().darken(),
                    },
                ),
                dot.clone(),
            ]
        })
        .collect();
    words.pop();
    words
}

fn paint_galleys<'a>(
    mut galleys: Vec<Arc<Galley>>,
    ui: &mut egui::Ui,
    centered: bool,
) -> egui::Response {
    galleys.reverse();

    let mut staging_galleys: Vec<Arc<Galley>> = vec![];

    let avail_width = ui.available_width();
    let origin = ui.next_widget_position();
    let word_height = galleys.first().unwrap().mesh_bounds.height();
    let word_pad = 4.0;
    let word_offset = 8.0;

    let mut current_row = 0;
    let mut current_width = 0.0;

    while !galleys.is_empty() || !staging_galleys.is_empty() {
        let last_galley = galleys.last().map(|galley| galley.mesh_bounds.width());
        let next_width = last_galley.unwrap_or_default() + word_pad * 2.0;

        if last_galley.is_none()
            || (current_width > 0.0 && current_width + next_width > avail_width)
        {
            let padding = if centered {
                (avail_width - current_width) / 2.0
            } else {
                0.0
            };
            let mut total_x = 0.0;
            for galley in staging_galleys.drain(0..) {
                let Vec2 { x, y } = galley.mesh_bounds.size();
                let word_pt = origin
                    + vec2(
                        padding + total_x + word_pad + word_offset,
                        current_row as f32 * y
                            + current_row as f32 * word_pad
                            + galley.mesh_bounds.height() * 0.25,
                    );

                ui.painter().galley(word_pt, galley);

                total_x += x + word_pad * 2.0;
            }
            current_row += 1;
            current_width = 0.0;
        } else {
            let galley = galleys.pop().unwrap();
            current_width += galley.mesh_bounds.width() + word_pad * 2.0;
            staging_galleys.push(galley);
        }
    }

    let total_height = word_height * current_row as f32;
    let battle_rect = epaint::Rect::from_min_size(origin, vec2(avail_width, total_height));

    ui.allocate_rect(battle_rect, Sense::click())
}

impl<'a> BattleUI<'a> {
    pub fn render(self, ctx: &GameCtx, ui: &mut egui::Ui) {
        let mut theme = ctx.theme.rescale(0.5);
        theme.tile_margin = 0.0;

        let mut battle_rect = Rect::NOTHING;

        ui.allocate_ui_with_layout(
            vec2(ui.available_size_before_wrap().x, 0.0),
            Layout::left_to_right(Align::Center).with_main_wrap(true),
            |ui| {
                let words = get_galleys(&self.battle.attackers, ctx, ui);
                battle_rect = battle_rect.union(paint_galleys(words, ui, false).rect);
            },
        );
        ui.add_space(5.0);

        let (msg, border_color) = match self.battle.outcome {
            truncate_core::judge::Outcome::AttackerWins(_) => {
                ("won an attack against", ctx.theme.addition.darken())
            }
            truncate_core::judge::Outcome::DefenderWins => {
                ("failed an attack against", ctx.theme.defeated.darken())
            }
        };
        let galley = ui.painter().layout_no_wrap(
            msg.to_string(),
            FontId::new(
                ctx.theme.letter_size * 0.3,
                egui::FontFamily::Name("Truncate-Heavy".into()),
            ),
            ctx.theme.text,
        );
        battle_rect = battle_rect.union(paint_galleys(vec![galley], ui, false).rect);
        ui.add_space(5.0);

        ui.allocate_ui_with_layout(
            vec2(ui.available_size_before_wrap().x, 0.0),
            Layout::left_to_right(Align::Center).with_main_wrap(true),
            |ui| {
                let words = get_galleys(&self.battle.defenders, ctx, ui);
                battle_rect = battle_rect.union(paint_galleys(words, ui, false).rect);
            },
        );

        let battle_interact = ui.interact(battle_rect, ui.auto_id_with("battle"), Sense::click());
        if battle_interact.hovered() {
            ui.painter().rect_stroke(
                battle_rect.expand2(vec2(0.0, 4.0)),
                2.0,
                Stroke::new(2.0, border_color),
            );
            ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
        }

        battle_rect.set_right(battle_rect.left() + 6.0);
        battle_rect = battle_rect.expand2(vec2(0.0, 4.0));
        ui.painter().rect_filled(battle_rect, 0.0, border_color)
    }
}