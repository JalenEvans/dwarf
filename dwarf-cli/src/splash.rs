/// Dwarven Warhammer ASCII art splash screen.
/// Embedded as a Rust string literal — no external file dependencies.
pub const SPLASH_ART: &str = r#"
                      __   __/\__   __
                     |  [_]      [_]  |
                     |     \____/     |
                    [       ____       ]
                     |   _ /    \ _   |  
                     |__[ ]_    _[ ]__|
                            \--/   
                            |--|                             
                            [--]                       
                             ||                       
                             ||                       
                             ||                       
                             ||                       
                             ||                       
                            [--]
"#;

/// Returns the full splash screen with version.
pub fn splash_screen(version: &str) -> String {
    format!("{}Dwarf Compiler v{}\n", SPLASH_ART, version)
}
