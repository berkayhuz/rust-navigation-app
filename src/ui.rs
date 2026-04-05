use macroquad::prelude::*;

#[derive(Clone, Copy)]
pub struct Button {
    pub rect: Rect,
    pub label: &'static str,
}

impl Button {
    pub fn new(x: f32, y: f32, w: f32, h: f32, label: &'static str) -> Self {
        Self {
            rect: Rect::new(x, y, w, h),
            label,
        }
    }

    pub fn hovered(&self) -> bool {
        let (mx, my) = mouse_position();
        self.rect.contains(vec2(mx, my))
    }

    pub fn clicked(&self) -> bool {
        self.hovered() && is_mouse_button_pressed(MouseButton::Left)
    }

    pub fn draw(&self, active: bool) {
        let bg = if active {
            Color::from_rgba(76, 129, 255, 255)
        } else if self.hovered() {
            Color::from_rgba(49, 59, 79, 255)
        } else {
            Color::from_rgba(30, 37, 50, 255)
        };

        draw_rectangle(self.rect.x, self.rect.y, self.rect.w, self.rect.h, bg);
        draw_rectangle_lines(
            self.rect.x,
            self.rect.y,
            self.rect.w,
            self.rect.h,
            1.0,
            Color::from_rgba(90, 101, 124, 255),
        );

        let m = measure_text(self.label, None, 22, 1.0);
        draw_text(
            self.label,
            self.rect.x + (self.rect.w - m.width) * 0.5,
            self.rect.y + self.rect.h * 0.62,
            22.0,
            WHITE,
        );
    }
}

pub fn draw_panel(x: f32, y: f32, w: f32, h: f32) {
    draw_rectangle(x, y, w, h, Color::from_rgba(18, 24, 35, 255));
    draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(67, 78, 95, 255));
}