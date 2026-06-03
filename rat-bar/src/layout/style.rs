use std::{borrow::Cow, collections::HashMap, convert::Infallible, ops::Deref, str::FromStr};

use lazy_static::lazy_static;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use regex::Captures;
use serde_json::Value;

use crate::app::State;

pub struct ParseStyle(pub ratatui::style::Style);
impl FromStr for ParseStyle {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ParseStyle(parse_style(s)))
    }
}
impl Deref for ParseStyle {
    type Target = Style;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
lazy_static! {
    static ref VARIABLES: regex::Regex = regex::Regex::new(r"\$\{(?<var>[^${}]*)\}").unwrap();
    static ref FORMAT: regex::Regex =
        regex::Regex::new(r"\$\[(?<args>[^\[\]]*)\]\((?<text>[^)\\]*(?:\\.[^)\\]*)*)\)").unwrap();
}

pub fn interpolate_string<'a>(string: &'a str, state: &State) -> String {
    VARIABLES
        .replace_all(string, |captures: &Captures| {
            let var = captures.name("var").unwrap();
            state
                .providers
                .variables
                .get(var.as_str())
                .map(|var| {
                    if let Value::String(string) = &var {
                        Cow::Borrowed(string.as_str())
                    } else {
                        Cow::Owned(var.to_string())
                    }
                })
                .unwrap_or(Cow::Borrowed("UNDEFINED"))
        })
        // replace normal spaces with u+2002 because kitty is weird
        // otherwise 2 width symbols render as 1 width on resize
        // completely ruins the appearance of nerd fonts
        .replace(char::is_whitespace, " ")
}

pub fn parse_style(str: &str) -> Style {
    let styles = str.split(',');

    styles.fold(Style::default(), |style, str| match str.split_once(':') {
        Some((str, args)) => match str.trim() {
            "bg" => Color::from_str(args.trim())
                .map(|c| style.bg(c))
                .unwrap_or(style),
            "fg" => Color::from_str(args.trim())
                .map(|c| style.fg(c))
                .unwrap_or(style),
            _ => style,
        },
        None => match str.trim() {
            "ul" => style.underlined(),
            "rv" => style.reversed(),
            "it" => style.italic(),
            "bo" => style.bold(),
            "sb" => style.slow_blink(),
            "rb" => style.rapid_blink(),
            "cr" => style.crossed_out(),
            "dm" => style.dim(),
            "hd" => style.hidden(),
            _ => style,
        },
    })
}

pub fn style_string<'a>(string: Cow<'a, str>) -> Line<'a> {
    let mut start = 0;
    let mut line = Line::default();
    for captures in FORMAT.captures_iter(string.as_ref()) {
        let match_start = captures.get_match().start();
        let style = captures.name("args").unwrap();
        let text = captures.name("text").unwrap();
        let span = Span::from(text.as_str().replace(r"\)", ")").replace(r"\(", "("))
            .style(parse_style(style.as_str()));

        if match_start > start {
            line.push_span(
                string[start..match_start]
                    .replace(r"\)", ")")
                    .replace(r"\(", "("),
            );
        }
        line.push_span(span);

        start = captures.get_match().end()
    }
    if start < string.len() {
        line.push_span(
            string[start..string.len()]
                .replace(r"\)", ")")
                .replace(r"\(", "("),
        );
    }

    line
}
