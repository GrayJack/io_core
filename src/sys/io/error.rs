pub type RawOsError = cfg_select! {
    target_os = "uefi" => usize,
    _ => i32,
};
