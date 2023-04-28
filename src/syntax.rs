use color_eyre::eyre::Result;
use lazy_static::lazy_static;
use std::io;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, Theme, ThemeSet},
    parsing::{SyntaxDefinition, SyntaxSet, SyntaxSetBuilder},
    util::as_24_bit_terminal_escaped,
};

lazy_static! {
    static ref THEME: Theme = {
        let theme_data: &'static [u8] = include_bytes!("MonokaiDarkSoda.tmTheme");
        let mut theme_reader: io::Cursor<&[u8]> = io::Cursor::new(theme_data);
        ThemeSet::load_from_reader(&mut theme_reader).unwrap()
    };

    // This should always result in Some(syntax_reference) since we
    // have explicitly sourced a PowerShell sublime-syntax file.

    static ref SYNTAX: SyntaxSet = {
    let syntax: SyntaxDefinition = SyntaxDefinition::load_from_str(
            include_str!("PowerShellSyntax.sublime-syntax"),
            false,
            None,
        ).unwrap();
        let mut syntax_set_builder = SyntaxSetBuilder::new();
        syntax_set_builder.add(syntax);
        syntax_set_builder.build()
    };
}

pub fn highlight(s: &str) -> Result<String> {
    let mut h = HighlightLines::new(SYNTAX.find_syntax_by_name("PowerShell").unwrap(), &THEME);
    let ranges: Vec<(Style, &str)> = h.highlight_line(s, &SYNTAX)?;
    Ok(as_24_bit_terminal_escaped(&ranges[..], false))
}
