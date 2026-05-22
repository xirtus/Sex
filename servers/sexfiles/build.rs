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
    println!("cargo:rerun-if-env-changed=SEXOS_LINEN_DISK_OBJECT_PROOF");
    println!("cargo:rerun-if-env-changed=SEXFILES_DISKFS_100_PROOF");
    println!("cargo:rustc-check-cfg=cfg(sexfiles_diskfs100_ap2_proof)");
    if std::env::var("SEXFILES_DISKFS_100_PROOF").as_deref() == Ok("1") {
        println!("cargo:rustc-cfg=sexfiles_diskfs100_ap2_proof");
    }
    println!("cargo:rerun-if-env-changed=SEXFILES_DISKFS_100_AP3_PROOF");
    println!("cargo:rustc-check-cfg=cfg(sexfiles_diskfs100_ap3_proof)");
    println!("cargo:rustc-check-cfg=cfg(sexfiles_diskfs_multi_object_proof)");
    if std::env::var("SEXFILES_DISKFS_100_AP3_PROOF").as_deref() == Ok("1") {
        println!("cargo:rustc-cfg=sexfiles_diskfs100_ap3_proof");
        println!("cargo:rustc-cfg=sexfiles_diskfs_multi_object_proof");
    }
}
