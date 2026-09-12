// VoiceType - AI Voice Input Tool
// Prevents additional console instance on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    voice_type_lib::run()
}
