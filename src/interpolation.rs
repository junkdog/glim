use std::f32::consts::PI;

use ratatui::style::{Color, Style};

#[derive(Clone)]
pub enum Interpolation {
    Linear,
    Smooth,
    BounceIn(u16),
    BounceOut(u16),
    // SmoothStep2,
    // Cubic,
    // CatmullRom,
    Pow(u32),
    PowIn(u32),
    PowOut(u32),
    Elastic(f32, f32, f32, u16), // value, power, scale, cycles
    // Bounce(u32),
}

impl Interpolation {

    pub fn alpha(&self, a: f32) -> f32 {
        match self {
            Interpolation::Linear                => a,
            Interpolation::Smooth                => a * a * (3.0 - 2.0 * a),
            Interpolation::BounceIn(bounces)     => 1.0 - Self::bounce_out(a, *bounces),
            Interpolation::BounceOut(bounces)    => Self::bounce_out(a, *bounces),
            Interpolation::Pow(p)                => Self::pow(a, *p),
            Interpolation::PowIn(p)              => Self::pow_in(a, *p),
            Interpolation::PowOut(p)             => Self::pow_out(a, *p),
            Interpolation::Elastic(v, p, amp, c) => Self::elastic(a, *v, *p, *amp, *c),
        }
    }

    fn pow(a: f32, power: u32) -> f32 {
        if a <= 0.5 {
            (a * 2.0).powf(power as f32 - 1.0) / 2.0
        } else {
            ((a - 1.0) * 2.0).powf(power as f32) / if power % 2 == 0 { -2.0 } else { 2.0 } + 1.0
        }
    }

    fn pow_in(a: f32, power: u32) -> f32 {
        a.powf(power as f32)
    }

    fn pow_out(a: f32, power: u32) -> f32 {
        (a - 1.0).powf(power as f32) * if power % 2 == 0 { -1.0 } else { 1.0 } + 1.0
    }

    fn elastic(
        a: f32,
        value: f32,
        power: f32,
        scale: f32,
        cycles: u16
    ) -> f32 {
        let bounces = cycles as f32 * PI * (if cycles % 2 == 0 { 1.0 } else { -1.0 });
        if a <= 0.5 {
            let alpha = a * 2.0;
            let exponent = power * (alpha - 1.0);
            value.powf(exponent) * (alpha * bounces).sin() * scale / 2.0
        } else {
            let alpha = 1.0 - a;
            let alpha = alpha * 2.0;

            let exponent = power * (alpha - 1.0);
            1.0 - value.powf(exponent) * (alpha * bounces).sin() * scale / 2.0
        }
    }


    fn bounce_out(
        t: f32,
        bounces: u16,
    ) -> f32 {
        let segments = 2.0 * bounces as f32 + 1.0; // Total segments including bounces and pauses.
        let segment_duration = 1.0 / segments;
        let current_segment = (t * segments).floor() as u16;

        if current_segment >= bounces * 2 {
            return 1.0; // Last segment, animation has completed.
        }

        let t_rel = (t - segment_duration * current_segment as f32) / segment_duration;
        let intensity = 2.0_f32.powf(-(current_segment as f32 / 2.0)); // Intensity decreases with each bounce.

        if current_segment % 2 == 0 { // bounce up
            1.0 - (intensity * (1.0 - t_rel * t_rel))
        } else {                      // bounce down
            1.0 - intensity + intensity * t_rel * t_rel
        }
    }
}

pub trait Interpolatable<T> {
    fn lerp(&self, target: &T, alpha: f32) -> T;
    
    fn tween(&self, target: &T, alpha: f32, interpolation: Interpolation) -> T {
        self.lerp(target, interpolation.alpha(alpha))
    }
}

impl Interpolatable<u16> for u16 {
    fn lerp(&self, target: &u16, alpha: f32) -> u16 {
        (*self as f32).lerp(
            &(*target as f32),
            alpha
        ).round() as u16
    }
}

impl Interpolatable<f32> for f32 {
    fn lerp(&self, target: &f32, alpha: f32) -> f32 {
        self + (target - self) * alpha
    }
}

impl Interpolatable<Style> for Style {
    fn lerp(&self, target: &Style, alpha: f32) -> Style {
        let fg = self.fg.lerp(&target.fg, alpha);
        let bg = self.bg.lerp(&target.bg, alpha);

        let mut s = *self;
        if let Some(fg) = fg { s = s.fg(fg) }
        if let Some(bg) = bg { s = s.bg(bg) }

        s
    }
}

impl Interpolatable<Color> for Color {
    fn lerp(&self, target: &Color, alpha: f32) -> Color {
        if alpha == 0.0 {
            return *self;
        } else if alpha == 1.0 {
            return *target;
        }
        
        let (h, s, v) = self.to_hsv();
        let (h2, s2, v2) = target.to_hsv();
        Color::from_hsv(
            h.lerp(&h2, alpha),
            s.lerp(&s2, alpha),
            v.lerp(&v2, alpha),
        )
    }
}

impl Interpolatable<Option<Color>> for Option<Color> {
    fn lerp(&self, target: &Option<Color>, alpha: f32) -> Option<Color> {
        match (self, target) {
            (Some(c1), Some(c2)) => Some(c1.lerp(c2, alpha)),
            (Some(c1), None)     => Some(*c1),
            (None,     Some(c2)) => Some(*c2),
            (None,     None)     => None,
        }
    }
}

trait HsvConvertable {
    fn from_hsv(h: f32, s: f32, v: f32) -> Self;
    fn to_hsv(&self) -> (f32, f32, f32);
}

impl HsvConvertable for Color {
    fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let hsl = colorsys::Hsl::new(h as f64, s as f64, v as f64, None);
        let color: colorsys::Rgb = hsl.as_ref().into();
        
        let red = color.red().round();
        let green = color.green().round();
        let blue = color.blue().round();
        
        Color::Rgb(red as u8, green as u8, blue as u8)
    }

    fn to_hsv(&self) -> (f32, f32, f32) {
        match self {
            Color::Rgb(r, g, b) => {
                let rgb = colorsys::Rgb::from([*r, *g, *b]);
                let hsl: colorsys::Hsl = rgb.as_ref().into();
                (hsl.hue() as f32, hsl.saturation() as f32, hsl.lightness() as f32)
            }
            _ => (0.0, 0.0, 0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn sandbox() {
        let samples: Vec<f32> = (0..100).map(|x| x as f32 / 99.0).collect();
        let results: Vec<f32> = samples.iter().map(|&t| Interpolation::bounce_out(t, 4)).collect();

        for &result in results.iter() {
            println!("{:.4}", result);
        }
    }

    #[test]
    fn test_color_from_hsv() {
        let color = Color::from_hsv(0.0, 1.0, 1.0);
        assert_eq!(color, Color::from_u32(0xFF0000));
    }

    #[test]
    fn test_color_from_hsv_2() {
        let color = Color::from_hsv(123.0, 0.14, 0.56);
        assert_eq!(color, Color::from_u32(0x7B8F7C));
    }

    #[test]
    fn test_color_to_hsv() {
        let color = Color::from_u32(0xFF0000);
        let (h, s, v) = color.to_hsv();
        assert_eq!(h, 0.0);
        assert_eq!(s, 1.0);
        assert_eq!(v, 1.0);
    }

    #[test]
    fn test_color_to_hsv_2() {
        let color = Color::from_u32(0x7B8F7C);
        let (h, s, v) = color.to_hsv();
        assert_eq!(h, 123.0);
        // assert_eq!(s, 0.14); // 0.13986018
        // assert_eq!(v, 0.56); // 0.56078434
    }

}