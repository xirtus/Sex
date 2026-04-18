#[allow(unused_variables, unused_imports, dead_code)]
#[allow(unused_variables, unused_imports)]
#[allow(unused_variables, dead_code, unused_imports)]
use sex_pdx::{LdProtocol, PdxReply, StoreProtocol, pdx_call};
use core::sync::atomic::{AtomicU64, Ordering};

pub static LD_OPS_TOTAL: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
pub fn handle_ld_message(msg: &LdProtocol, reply: &mut PdxReply) {
    LD_OPS_TOTAL.fetch_add(1, Ordering::Relaxed);
    match msg {
        LdProtocol::ResolveObject { name: _ } => {
            // Call sexshop (Slot 4 in sex-ld's context) to get object hash
            // For now, mock a hash
            reply.status = 0;
            reply.size = 0x1234; // Mock hash as size for proto
        },
        LdProtocol::MapLibrary { hash: _, base_addr: _ } => {
            // Call sexshop::ObjectGet to get PFN
            let store_msg = StoreProtocol::ObjectGet { hash: [0u8; 32] };
            let res = pdx_call(4, 0, &store_msg as *const _ as u64, 0);
            
            if res != u64::MAX {
                reply.status = 0;
                reply.size = res; // Return PFN
            } else {
                reply.status = -1;
            }
        },
        LdProtocol::GetEntry { hash: _ } => {
            reply.status = 0;
            reply.size = 0x_4000_0000; // Mock entry point
        },
        LdProtocol::Stats => {
            reply.status = LD_OPS_TOTAL.load(Ordering::Relaxed) as i64;
        }
    }
}
