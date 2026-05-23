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
    println!("cargo:rerun-if-env-changed=SEXFILES_DISKFS_100_AP4_WRITE");
    println!("cargo:rerun-if-env-changed=SEXFILES_DISKFS_100_AP4_READ");
    println!("cargo:rustc-check-cfg=cfg(sexfiles_diskfs100_ap4_write)");
    println!("cargo:rustc-check-cfg=cfg(sexfiles_diskfs100_ap4_read)");
    if std::env::var("SEXFILES_DISKFS_100_AP4_WRITE").as_deref() == Ok("1") {
        println!("cargo:rustc-cfg=sexfiles_diskfs100_ap4_write");
    }
    if std::env::var("SEXFILES_DISKFS_100_AP4_READ").as_deref() == Ok("1") {
        println!("cargo:rustc-cfg=sexfiles_diskfs100_ap4_read");
    }
    // AP5 negative test lanes
    println!("cargo:rerun-if-env-changed=SEXFILES_DISKFS_100_AP5_NEGATIVE");
    println!("cargo:rerun-if-env-changed=SEXFILES_DISKFS_100_AP5_NEG_MISMATCH");
    println!("cargo:rerun-if-env-changed=SEXFILES_DISKFS_100_AP5_NEG_MISSING_IMAGE");
    println!("cargo:rerun-if-env-changed=SEXFILES_DISKFS_100_AP5_NEG_READ_NO_WRITE");
    println!("cargo:rerun-if-env-changed=SEXFILES_DISKFS_100_AP5_NEG_FLUSH_SKIP");
    println!("cargo:rustc-check-cfg=cfg(sexfiles_diskfs100_ap5_neg_mismatch)");
    println!("cargo:rustc-check-cfg=cfg(sexfiles_diskfs100_ap5_neg_missing_image)");
    println!("cargo:rustc-check-cfg=cfg(sexfiles_diskfs100_ap5_neg_read_no_write)");
    println!("cargo:rustc-check-cfg=cfg(sexfiles_diskfs100_ap5_neg_flush_skip)");
    if std::env::var("SEXFILES_DISKFS_100_AP5_NEGATIVE").as_deref() == Ok("1") {
        if std::env::var("SEXFILES_DISKFS_100_AP5_NEG_MISMATCH").as_deref() == Ok("1") {
            println!("cargo:rustc-cfg=sexfiles_diskfs100_ap5_neg_mismatch");
        }
        if std::env::var("SEXFILES_DISKFS_100_AP5_NEG_MISSING_IMAGE").as_deref() == Ok("1") {
            println!("cargo:rustc-cfg=sexfiles_diskfs100_ap5_neg_missing_image");
        }
        if std::env::var("SEXFILES_DISKFS_100_AP5_NEG_READ_NO_WRITE").as_deref() == Ok("1") {
            println!("cargo:rustc-cfg=sexfiles_diskfs100_ap5_neg_read_no_write");
            // read-no-write reuses the AP4 read path: set both cfgs
            println!("cargo:rustc-cfg=sexfiles_diskfs100_ap4_read");
        }
    }
    if std::env::var("SEXFILES_DISKFS_100_AP5_NEG_FLUSH_SKIP").as_deref() == Ok("1") {
        println!("cargo:rustc-cfg=sexfiles_diskfs100_ap5_neg_flush_skip");
    }
}
