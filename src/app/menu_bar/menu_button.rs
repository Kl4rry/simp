use egui::NumExt;

#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct MenuButton {
    text: egui::WidgetText,
    tip: Option<egui::WidgetText>,
}

impl MenuButton {
    pub fn new(text: impl Into<egui::WidgetText>) -> Self {
        Self {
            text: text.into(),
            tip: None,
        }
    }

    pub fn tip(mut self, tip: impl Into<egui::WidgetText>) -> Self {
        self.tip = Some(tip.into());
        self
    }
}

impl egui::Widget for MenuButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let MenuButton { text, tip } = self;

        let button_padding = ui.spacing().button_padding;
        let total_extra = button_padding + button_padding;

        let wrap_width = ui.available_width() - total_extra.x;
        let text = text.into_galley(ui, Some(false), wrap_width, egui::TextStyle::Button);

        let mut desired_size = text.size() + 2.0 * button_padding;

        let tip = match tip {
            Some(tip) => {
                let tip = tip.into_galley(ui, Some(false), wrap_width, egui::TextStyle::Button);
                desired_size.x += tip.size().x + ui.spacing().icon_spacing;
                desired_size.y = desired_size.y.max(tip.size().y + 2.0 * button_padding.y);
                Some(tip)
            }
            None => None,
        };

        desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);

        let (rect, response) = ui.allocate_at_least(desired_size, egui::Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, text.text()));

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            let text_pos = ui
                .layout()
                .align_size_within_rect(text.size(), rect.shrink2(button_padding))
                .min;

            if ui.visuals().button_frame {
                let fill = visuals.bg_fill;
                let stroke = visuals.bg_stroke;
                ui.painter().rect(
                    rect.expand(visuals.expansion),
                    visuals.rounding,
                    fill,
                    stroke,
                );
            }

            let text_width = text.size().x;
            text.paint_with_visuals(ui.painter(), text_pos, visuals);

            if let Some(tip) = tip {
                let mut rect = rect.shrink2(button_padding);
                *rect.left_mut() += text_width;
                let pos = egui::Pos2::new(
                    rect.max.x - tip.size().x,
                    rect.min.y + ((rect.max.y - rect.min.y) - tip.size().y) / 2.0,
                );

                let text_color = if response.hovered() {
                    egui::Color32::GRAY
                } else {
                    egui::Color32::DARK_GRAY
                };

                ui.painter().add(egui::epaint::TextShape {
                    pos,
                    galley: tip.galley,
                    override_text_color: Some(text_color),
                    underline: egui::Stroke::none(),
                    angle: 0.0,
                });
            }
        }

        response
    }
}
