fn main() {
    println!("cargo:rerun-if-env-changed=SEXOS_LINEN_DISKFS_DIRECT_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_LINEN_DISKFS_SLOT_PROOF");
    println!("cargo:rerun-if-env-changed=SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP2");
    println!("cargo:rerun-if-env-changed=SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP3_WRITE");
    println!("cargo:rerun-if-env-changed=SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP3_READ");
}
