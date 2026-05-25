fn main() {
    println!("cargo:rerun-if-env-changed=SEXOS_LINEN_DISKFS_DIRECT_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_LINEN_DISKFS_SLOT_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP2");
    println!("cargo:rerun-if-env-changed=SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP3_WRITE");
    println!("cargo:rerun-if-env-changed=SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP3_READ");
    println!("cargo:rerun-if-env-changed=LINEN_REBOOT_RESTORE_CURRENT_TIER_PROOF");
    println!("cargo:rustc-check-cfg=cfg(linen_reboot_restore_current_tier_proof)");
    if std::env::var("LINEN_REBOOT_RESTORE_CURRENT_TIER_PROOF").as_deref() == Ok("1") {
        println!("cargo:rustc-cfg=linen_reboot_restore_current_tier_proof");
    }
}
