//! The Racebox's `Slider` rows in a front-end definition: HD's `TARGET` is one
//! (`<Slider name="Eliminations"> ... minSlide="200" maxSlide="600" incSlide="20"`).

/// The `minSlide` of the slider named `name`, or `None` where the definition authors none.
#[must_use]
pub fn slider_min(xml: &str, name: &str) -> Option<u32> {
    let at = xml.find(&format!("<Slider name=\"{name}\""))?;
    let body = &xml[at..];
    let body = &body[..body.find("</Slider>")?];
    body.split(" minSlide=\"").nth(1)?.split('"').next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::slider_min;

    const ROW: &str = r#"<List name="Other"></List><Slider name="Eliminations" focus="true">
        <Values idstring="IG_HUD_TARGET" minSlide="200" maxSlide="600" incSlide="20"></Values>
        </Slider><Slider name="NitroTarget"><Values minSlide="15"></Values></Slider>"#;

    #[test]
    fn the_named_slider_gives_its_own_minimum() {
        assert_eq!(slider_min(ROW, "Eliminations"), Some(200));
        assert_eq!(slider_min(ROW, "NitroTarget"), Some(15));
    }

    #[test]
    fn a_list_of_the_same_name_is_not_a_slider() {
        assert_eq!(slider_min(ROW, "Other"), None);
    }
}
