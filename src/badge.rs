extern crate base64;
extern crate chrono;
extern crate rusttype;

use base64::display::Base64Display;
use chrono::Duration;
use rusttype::{point, Font, FontCollection, Point, PositionedGlyph, Scale};

const FONT_DATA: &[u8] = include_bytes!("DejaVuSans.ttf");
const FONT_SIZE: f32 = 11.;

const MUTE: &str = r#"<path
     d="m 9.7196219,4.2202987 -2.7802854,2.7796601 -3.1893438,0 C 3.3356217,6.9999588 3,7.3355788 3,7.7499488 l 0,4.4999592 c 0,0.41405 0.3356217,0.74999 0.7499927,0.74999 l 3.1893438,0 2.7802854,2.77966 c 0.4696831,0.46968 1.2803001,0.13969 1.2803001,-0.53031 l 0,-10.4986493 c 0,-0.67061 -0.811242,-0.99936 -1.2803001,-0.5303 z m 7.7064871,5.7796295 1.426236,-1.426239 c 0.196873,-0.19687 0.196873,-0.51624 0,-0.7131204 l -0.713118,-0.71311 c -0.196873,-0.19688 -0.516245,-0.19688 -0.713118,0 L 15.999873,8.5736892 14.573637,7.1474588 c -0.196873,-0.19688 -0.516245,-0.19688 -0.713118,0 l -0.713118,0.71311 c -0.196873,0.1968804 -0.196873,0.5162504 0,0.7131204 l 1.426236,1.426239 -1.425924,1.4259198 c -0.196873,0.19688 -0.196873,0.51625 0,0.71312 l 0.713118,0.71312 c 0.196874,0.19687 0.516245,0.19687 0.713118,0 l 1.425924,-1.42593 1.426236,1.42624 c 0.196873,0.19687 0.516245,0.19687 0.713118,0 l 0.713118,-0.71312 c 0.196873,-0.19687 0.196873,-0.51624 0,-0.71312 L 17.426109,9.9999282 Z"
     />"#;

pub const COLOR_DANGER: &str = "#eb364b";
pub const COLOR_WARNING: &str = "#ffb52b";
pub const COLOR_SUCCESS: &str = "#41c464";
pub const COLOR_OTHER: &str = "#949196";

#[derive(Clone, Debug, PartialEq)]
pub struct BadgeOptions {
    /// Status will be displayed on the left side of badge
    pub status: String,
    /// Duration will be displayed on the right side of badge
    pub duration: Option<Duration>,
    /// HTML color of badge
    pub color: String,
    /// Is the Badge muted
    pub muted: bool,
    /// The image width, defaults to the size of the badge
    pub width: Option<u32>,
    /// The image height, defaults to the size of the badge
    pub height: Option<u32>,
}

impl Default for BadgeOptions {
    fn default() -> BadgeOptions {
        BadgeOptions {
            status: "Ok".to_owned(),
            duration: None,
            color: "#4c1".to_owned(),
            muted: false,
            width: None,
            height: None,
        }
    }
}

pub struct Badge {
    options: BadgeOptions,
    font: Font<'static>,
    scale: Scale,
    offset: Point<f32>,
}

impl Badge {
    pub fn new(options: BadgeOptions) -> Badge {
        let collection = FontCollection::from_bytes(FONT_DATA);
        // this should never fail in practice
        let font = collection.unwrap().into_font().unwrap();
        let scale = Scale {
            x: FONT_SIZE,
            y: FONT_SIZE,
        };
        let v_metrics = font.v_metrics(scale);
        let offset = point(0.0, v_metrics.ascent);
        Badge {
            options,
            font,
            scale,
            offset,
        }
    }

    pub fn to_svg_data_uri(&self) -> String {
        format!(
            "data:image/svg+xml;base64,{}",
            Base64Display::with_config(self.to_svg().as_bytes(), base64::STANDARD)
        )
    }

    fn human_str(d: &Duration) -> String {
        let weeks = d.num_weeks();
        if weeks > 1 {
            return format!("{} weeks", weeks);
        } else if weeks > 0 {
            return format!("{} week", weeks);
        }
        let days = d.num_days();
        if days > 1 {
            return format!("{} days", days);
        } else if days > 0 {
            return format!("{} day", days);
        }
        let hours = d.num_hours();
        if hours > 1 {
            return format!("{} hours", hours);
        } else if hours > 0 {
            return format!("{} hour", hours);
        }
        let minutes = d.num_minutes();
        if minutes > 1 {
            return format!("{} minutes", minutes);
        } else if minutes > 0 {
            return format!("{} minute", minutes);
        }
        let seconds = d.num_seconds();
        if seconds != 1 {
            return format!("{} seconds", seconds);
        } else {
            return format!("{} second", seconds);
        }
    }

    pub fn to_svg(&self) -> String {
        let duration = match &self.options.duration {
            Some(v) => Self::human_str(v),
            None => "n/a".to_owned(),
        };
        let left_width = self.calculate_width(&self.options.status) + 6;
        let right_width = self.calculate_width(&duration) + 6;
        let offset = if self.options.muted { 20 } else { 0 };

        let svg = format!(
            r###"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{}" height="{}">
  <linearGradient id="smooth" x2="0" y2="100%">
    <stop offset="0" stop-color="#bbb" stop-opacity=".1"/>
    <stop offset="1" stop-opacity=".1"/>
  </linearGradient>

  <mask id="round">
    <rect width="{}" height="20" rx="3" fill="#fff"/>
  </mask>

  <g mask="url(#round)">
    <rect width="{}" height="20" fill="{}"/>
    <rect x="{}" width="{}" height="20" fill="#555"/>
    <rect width="{}" height="20" fill="url(#smooth)"/>
  </g>

  <g fill="#fff" text-anchor="middle" font-family="DejaVu Sans,Verdana,Geneva,sans-serif" font-size="11">
    {}
    <text x="{}" y="15" fill="#010101" fill-opacity=".3">{}</text>
    <text x="{}" y="14">{}</text>
    <text x="{}" y="15" fill="#010101" fill-opacity=".3">{}</text>
    <text x="{}" y="14">{}</text>
  </g>
</svg>"###,
            self.options
                .width
                .unwrap_or_else(|| offset + left_width + right_width),
            self.options.height.unwrap_or(20),
            offset + left_width + right_width,
            offset + left_width,
            self.options.color,
            offset + left_width,
            right_width,
            offset + left_width + right_width,
            if self.options.muted { MUTE } else { "" },
            offset + (left_width) / 2,
            &self.options.status,
            offset + (left_width) / 2,
            &self.options.status,
            offset + left_width + (right_width / 2),
            &duration,
            offset + left_width + (right_width / 2),
            &duration
        );

        svg
    }

    fn calculate_width(&self, text: &str) -> u32 {
        let glyphs: Vec<PositionedGlyph> =
            self.font.layout(text, self.scale, self.offset).collect();
        let width: u32 = glyphs
            .iter()
            .rev()
            .filter_map(|g| {
                g.pixel_bounding_box()
                    .map(|b| b.min.x as f32 + g.unpositioned().h_metrics().advance_width)
            })
            .next()
            .unwrap_or(0.0)
            .ceil() as u32;
        width + ((text.len() as u32 - 1) * 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> BadgeOptions {
        BadgeOptions::default()
    }

    #[test]
    fn test_calculate_width() {
        let badge = Badge::new(options());
        assert_eq!(badge.calculate_width("build"), 31);
        assert_eq!(badge.calculate_width("passing"), 48);
    }

    #[test]
    #[ignore]
    fn test_to_svg() {
        use std::fs::File;
        use std::io::Write;
        let mut file = File::create("test.svg").unwrap();
        let options = BadgeOptions {
            duration: Some(Duration::hours(16)),
            status: "ALERT".to_owned(),
            muted: true,
            color: COLOR_WARNING.to_string(),
            ..BadgeOptions::default()
        };
        let badge = Badge::new(options);
        file.write_all(badge.to_svg().as_bytes()).unwrap();
    }
}
