use crate::elements::field_orientation::FieldOrientation;
use crate::elements::text_field::TextField;

#[derive(Clone)]
pub struct DrawerState {
    pub auto_pos_x: f64,
    pub auto_pos_y: f64,
}

impl Default for DrawerState {
    fn default() -> Self {
        DrawerState {
            auto_pos_x: 0.0,
            auto_pos_y: 0.0,
        }
    }
}

impl DrawerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_automatic_text_position(&mut self, text: &TextField, w: f64) {
        if !text.position.calculate_from_bottom {
            return;
        }

        let x = text.position.x as f64;
        let y = text.position.y as f64;

        if !text.position.automatic_position {
            self.auto_pos_x = x;
            self.auto_pos_y = y;
        }

        match text.font.orientation {
            FieldOrientation::Rotated90 => self.auto_pos_y += w,
            FieldOrientation::Rotated180 => self.auto_pos_x -= w,
            FieldOrientation::Rotated270 => self.auto_pos_y -= w,
            _ => self.auto_pos_x += w,
        }
    }

    pub fn get_text_position(&self, text: &TextField) -> (f64, f64) {
        if text.position.automatic_position {
            (self.auto_pos_x, self.auto_pos_y)
        } else {
            (text.position.x as f64, text.position.y as f64)
        }
    }
}
