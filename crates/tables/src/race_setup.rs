//! The Racebox's `Slider` rows in a front-end definition: HD's `TARGET` is one
//! (`<Slider name="Eliminations"> ... minSlide="200" maxSlide="600" incSlide="20"`).

/// A `Slider`'s authored range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slider {
    /// `minSlide`: where the row starts, and what an untouched row races on.
    pub min: u32,
    /// `maxSlide`.
    pub max: u32,
    /// `incSlide`: what one press moves it by.
    pub step: u32,
}

impl Slider {
    /// Every value the row steps through, `min` to `max` inclusive. A zero step offers `min`
    /// alone rather than looping.
    #[must_use]
    pub fn values(&self) -> Vec<u32> {
        let step = self.step.max(1) as usize;
        (self.min..=self.max.max(self.min)).step_by(step).collect()
    }
}

/// The slider named `name`, or `None` where the definition authors none or leaves out `minSlide`.
/// A missing `maxSlide` or `incSlide` leaves the row a single value, its minimum.
#[must_use]
pub fn slider(xml: &str, name: &str) -> Option<Slider> {
    let at = xml.find(&format!("<Slider name=\"{name}\""))?;
    let body = &xml[at..];
    let body = &body[..body.find("</Slider>")?];
    let number = |key: &str| {
        body.split(&format!(" {key}=\""))
            .nth(1)?
            .split('"')
            .next()?
            .parse()
            .ok()
    };
    let min = number("minSlide")?;
    Some(Slider {
        min,
        max: number("maxSlide").unwrap_or(min),
        step: number("incSlide").unwrap_or(1),
    })
}

#[cfg(test)]
mod tests {
    use super::{Slider, slider};

    const ROW: &str = r#"<List name="Other"></List><Slider name="Eliminations" focus="true">
        <Values idstring="IG_HUD_TARGET" minSlide="200" maxSlide="600" incSlide="20"></Values>
        </Slider><Slider name="NitroTarget"><Values minSlide="15"></Values></Slider>"#;

    #[test]
    fn the_named_slider_gives_its_own_range() {
        let target = slider(ROW, "Eliminations").unwrap();
        assert_eq!(
            target,
            Slider {
                min: 200,
                max: 600,
                step: 20
            }
        );
        assert_eq!(slider(ROW, "NitroTarget").map(|s| s.min), Some(15));
    }

    #[test]
    fn a_slider_steps_from_its_minimum_to_its_maximum() {
        let values = slider(ROW, "Eliminations").unwrap().values();
        assert_eq!(
            (values.len(), values[0], values[1], values[20]),
            (21, 200, 220, 600)
        );
        assert_eq!(slider(ROW, "NitroTarget").unwrap().values(), [15]);
    }

    #[test]
    fn a_list_of_the_same_name_is_not_a_slider() {
        assert_eq!(slider(ROW, "Other"), None);
    }
}
