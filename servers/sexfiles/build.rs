fn main() {
    println!("cargo:rerun-if-env-changed=SEXFILES_RAMFS_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_DISKFS_OBJECT_TABLE_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_SEXFILES_JOURNAL_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_SEXFILES_REPLAY_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_SEXFILES_CAP_RECORD_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_LINEN_SEXFILES_METADATA_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_SEXFILES_FAULT_INJECTION_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_SEXFILES_REAL_BLOCK_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_SEXFILES_REBOOT_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_SEXFILES_EXTENT_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_SEXFILES_CHECKPOINT_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_SEXOBJECT_VIEW_PROOF");
}
