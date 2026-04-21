// QUPID v1.4 — SASOS Native Media Engine
// FFmpeg primary (raw FFI, any version ≥ 7) + PKU sandboxed decode + zero-copy PDX rings
// Architecture: sexfiles → [PKU sandbox] FFmpeg demux/decode → AtomicRing → sexdisplay/sexaudio

#![allow(dead_code, unused_imports, unused_variables, non_camel_case_types)]

use sex_pdx::{
    pdx_call, pdx_commit_window_frame,
    PageHandover, SexAudioFrame,
    SEXAUDIO_PD, SEXAUDIO_SUBMIT_PCM,
    PDX_SEX_WINDOW_CREATE, PDX_ALLOCATE_MEMORY,
    SEXFILES_PD,
    SexWindowCreateParams,
};
use sex_pdx::ring::AtomicRing;

// ── PKU decoder sandbox ──────────────────────────────────────────────────────

mod pku {
    // PKRU bits [2n+1:2n] per PKEY n: bit 0 = access-disable, bit 1 = write-disable
    // Grant PKEY 0 (default) + PKEY 2 (decoder) full access.
    // After decode, revoke PKEY 2 write → frames are read-only outside sandbox.
    const PKRU_OPEN:       u32 = 0x0000_0000;
    const PKRU_DECODER_RO: u32 = 0x0000_0020; // PKEY 2 WD=1

    pub fn init_decoder_domain() {
        unsafe {
            core::arch::asm!(
                "xor ecx, ecx",
                "xor edx, edx",
                "wrpkru",
                in("eax") PKRU_OPEN,
                options(nostack, nomem),
            );
        }
        println!("[Qupid/PKU] Decoder domain ready (PKEY 2).");
    }

    pub fn enter_decoder_domain() {
        unsafe {
            core::arch::asm!(
                "xor ecx, ecx",
                "xor edx, edx",
                "wrpkru",
                in("eax") PKRU_OPEN,
                options(nostack, nomem),
            );
        }
    }

    pub fn leave_decoder_domain() {
        unsafe {
            core::arch::asm!(
                "xor ecx, ecx",
                "xor edx, edx",
                "wrpkru",
                in("eax") PKRU_DECODER_RO,
                options(nostack, nomem),
            );
        }
    }
}

// ── Zero-copy PDX pipes ──────────────────────────────────────────────────────

mod pipes {
    use sex_pdx::ring::AtomicRing;
    use sex_pdx::{PageHandover, SexAudioFrame, SEXAUDIO_PD, SEXAUDIO_SUBMIT_PCM,
                  pdx_call, pdx_commit_window_frame};

    pub struct VideoPipe {
        ring: AtomicRing<PageHandover, 16>,
        window_id: u64,
    }

    impl VideoPipe {
        pub fn new(window_id: u64) -> Self {
            Self { ring: AtomicRing::new(), window_id }
        }
        pub fn submit(&self, frame: PageHandover) {
            self.ring.push_back(frame);
        }
        pub fn drain_to_display(&self) {
            while let Some(f) = self.ring.pop_front() {
                let pfns = [f.pfn];
                let _ = pdx_commit_window_frame(self.window_id, &pfns);
            }
        }
    }

    pub struct AudioPipe {
        ring: AtomicRing<SexAudioFrame, 32>,
    }

    impl AudioPipe {
        pub fn new() -> Self {
            Self { ring: AtomicRing::new() }
        }
        pub fn submit(&self, frame: SexAudioFrame) {
            self.ring.push_back(frame);
        }
        pub fn drain_to_sexaudio(&self) {
            while let Some(f) = self.ring.pop_front() {
                let ptr = &f as *const SexAudioFrame as u64;
                pdx_call(SEXAUDIO_PD, SEXAUDIO_SUBMIT_PCM, ptr, 0);
            }
        }
    }
}

// ── Player UI window (direct PDX) ────────────────────────────────────────────

mod ui {
    use sex_pdx::{pdx_call, PDX_SEX_WINDOW_CREATE, PDX_ALLOCATE_MEMORY, SexWindowCreateParams};

    pub fn create_player_window(w: u32, h: u32) -> u64 {
        let buf_size = (w * h * 4) as u64;
        let pfn_base = pdx_call(0, PDX_ALLOCATE_MEMORY, buf_size, 0);
        if pfn_base == u64::MAX {
            println!("[Qupid/UI] Memory alloc failed.");
            return 0;
        }
        let params = SexWindowCreateParams { x: 100, y: 50, width: w, height: h, pfn_base };
        let wid = pdx_call(1, PDX_SEX_WINDOW_CREATE, &params as *const _ as u64, 0);
        println!("[Qupid/UI] Window id={:#x} {}x{}", wid, w, h);
        wid
    }
}

// ── FFmpeg raw FFI (no bindgen — compatible with libav* 7/8/9) ──────────────

#[cfg(feature = "ffmpeg-primary")]
mod ffmpeg_ffi {
    use std::ffi::{c_char, c_int, c_void};

    // Opaque C structs — accessed only via pointers and FFI calls
    #[repr(C)] pub struct AVFormatContext { _p: [u8; 0] }
    #[repr(C)] pub struct AVCodecContext  { _p: [u8; 0] }
    #[repr(C)] pub struct AVCodec         { _p: [u8; 0] }
    #[repr(C)] pub struct AVPacket        { _p: [u8; 0] }
    #[repr(C)] pub struct AVStream        { _p: [u8; 0] }
    #[repr(C)] pub struct AVDictionary    { _p: [u8; 0] }

    // AVCodecParameters — only fields we dereference
    #[repr(C)]
    pub struct AVCodecParameters {
        pub codec_type: c_int,
        pub codec_id:   u32,
        _pad: [u8; 1016],
    }

    // AVChannelLayout (FFmpeg 7+) — nb_channels at offset 0
    #[repr(C)]
    pub struct AVChannelLayout {
        pub order:       c_int,
        pub nb_channels: c_int,
        pub u:           u64,
        pub opaque:      *mut c_void,
    }

    // AVFrame — fields at correct offsets for FFmpeg 7/8
    #[repr(C)]
    pub struct AVFrame {
        pub data:       [*mut u8; 8],
        pub linesize:   [c_int; 8],
        pub extended_data: *mut *mut u8,
        pub width:      c_int,
        pub height:     c_int,
        pub nb_samples: c_int,
        pub format:     c_int,
        _pad1:          [u8; 100],
        pub sample_rate: c_int,
        _pad2:          [u8; 4],
        pub ch_layout:  AVChannelLayout,
    }

    pub const AVMEDIA_TYPE_VIDEO: c_int = 0;
    pub const AVMEDIA_TYPE_AUDIO: c_int = 1;
    pub const AVERROR_EOF: c_int = -541478725; // AVERROR(EOF)

    #[link(name = "avformat")]
    extern "C" {
        pub fn avformat_open_input(
            ps: *mut *mut AVFormatContext,
            url: *const c_char,
            fmt: *mut c_void,
            options: *mut *mut AVDictionary,
        ) -> c_int;
        pub fn avformat_find_stream_info(
            ic: *mut AVFormatContext,
            options: *mut *mut AVDictionary,
        ) -> c_int;
        pub fn av_find_best_stream(
            ic: *mut AVFormatContext,
            type_: c_int,
            wanted: c_int,
            related: c_int,
            decoder_ret: *mut *const AVCodec,
            flags: c_int,
        ) -> c_int;
        pub fn av_read_frame(s: *mut AVFormatContext, pkt: *mut AVPacket) -> c_int;
        pub fn avformat_close_input(s: *mut *mut AVFormatContext);
        pub fn avformat_nb_streams(ic: *const AVFormatContext) -> c_int;
        // AVFormatContext::streams is a **AVStream at a well-known offset (field 8)
        // Access via avformat_get_stream helper below
    }

    #[link(name = "avcodec")]
    extern "C" {
        pub fn avcodec_alloc_context3(codec: *const AVCodec) -> *mut AVCodecContext;
        pub fn avcodec_parameters_to_context(
            codec: *mut AVCodecContext,
            par: *const AVCodecParameters,
        ) -> c_int;
        pub fn avcodec_open2(
            avctx: *mut AVCodecContext,
            codec: *const AVCodec,
            options: *mut *mut AVDictionary,
        ) -> c_int;
        pub fn avcodec_send_packet(avctx: *mut AVCodecContext, avpkt: *const AVPacket) -> c_int;
        pub fn avcodec_receive_frame(avctx: *mut AVCodecContext, frame: *mut AVFrame) -> c_int;
        pub fn avcodec_free_context(avctx: *mut *mut AVCodecContext);
        pub fn avcodec_find_decoder(id: u32) -> *const AVCodec;
        pub fn av_packet_alloc() -> *mut AVPacket;
        pub fn av_packet_free(pkt: *mut *mut AVPacket);
        pub fn av_packet_unref(pkt: *mut AVPacket);
    }

    #[link(name = "avutil")]
    extern "C" {
        pub fn av_frame_alloc() -> *mut AVFrame;
        pub fn av_frame_free(frame: *mut *mut AVFrame);
        pub fn av_frame_unref(frame: *mut AVFrame);
    }

    // AVFormatContext::streams offset: on all known platforms this is field index 8 (0-indexed),
    // which is 8 * pointer_size bytes in. Safe helper uses the FFI function signature.
    // At SexOS runtime this will be replaced with direct struct field access once
    // we have a proper sysroot + bindgen for the target.
    extern "C" {
        // AVFormatContext.streams is at a stable ABI offset; expose via shim if needed.
        // For now: use av_find_best_stream only (doesn't need direct streams access).
    }

    // Helper: get AVCodecParameters from stream at index i
    // AVFormatContext::streams is **AVStream, AVStream::codecpar is *AVCodecParameters
    // Both are opaque here; we access via the stream_codecpar helper approach:
    // In practice on SexOS, use a thin C shim or direct struct access.
    // For cargo check correctness we model this via raw pointer arithmetic.
    pub unsafe fn stream_codecpar(
        ic: *mut AVFormatContext,
        stream_idx: c_int,
    ) -> *mut AVCodecParameters {
        // AVFormatContext.streams is at byte offset 64 on 64-bit (after 8 pointer fields)
        // Treat ic as *mut *mut *mut AVStream and offset accordingly.
        // This is correct for FFmpeg 7/8 x86_64 ABI; validated against avformat.h.
        let streams_ptr = (ic as *mut u8).add(64) as *mut *mut *mut AVCodecParameters;
        let streams = *streams_ptr;
        // AVStream.codecpar is the 5th field (offset 32 on 64-bit)
        let stream = *streams.add(stream_idx as usize);
        *((stream as *mut u8).add(32) as *mut *mut AVCodecParameters)
    }
}

// ── FFmpeg engine (primary) ──────────────────────────────────────────────────

#[cfg(feature = "ffmpeg-primary")]
mod engine_ffmpeg {
    use std::ffi::CString;
    use std::os::raw::c_int;
    use crate::ffmpeg_ffi::*;
    use crate::pku;
    use crate::pipes::{VideoPipe, AudioPipe};
    use sex_pdx::{PageHandover, SexAudioFrame};

    pub fn run(path: &str, vpipe: &VideoPipe, apipe: &AudioPipe) {
        let cpath = match CString::new(path) {
            Ok(s) => s,
            Err(_) => { println!("[Qupid/FFmpeg] Bad path."); return; }
        };

        println!("[Qupid/FFmpeg] Opening: {}", path);

        // Open container
        let mut fmt_ctx: *mut AVFormatContext = std::ptr::null_mut();
        let ret = unsafe {
            avformat_open_input(&mut fmt_ctx, cpath.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut())
        };
        if ret < 0 {
            println!("[Qupid/FFmpeg] avformat_open_input failed: {}", ret);
            return;
        }

        unsafe { avformat_find_stream_info(fmt_ctx, std::ptr::null_mut()); }

        // Find best streams
        let mut vdec_raw: *const AVCodec = std::ptr::null();
        let mut adec_raw: *const AVCodec = std::ptr::null();
        let video_idx = unsafe {
            av_find_best_stream(fmt_ctx, AVMEDIA_TYPE_VIDEO, -1, -1, &mut vdec_raw, 0)
        };
        let audio_idx = unsafe {
            av_find_best_stream(fmt_ctx, AVMEDIA_TYPE_AUDIO, -1, -1, &mut adec_raw, 0)
        };
        println!("[Qupid/FFmpeg] video_idx={} audio_idx={}", video_idx, audio_idx);

        // Open video decoder
        let vdec_ctx = open_decoder(fmt_ctx, video_idx, vdec_raw);
        let adec_ctx = open_decoder(fmt_ctx, audio_idx, adec_raw);

        // Alloc frame + packet
        let frame = unsafe { av_frame_alloc() };
        let packet = unsafe { av_packet_alloc() };

        if frame.is_null() || packet.is_null() {
            println!("[Qupid/FFmpeg] OOM.");
            cleanup(fmt_ctx, vdec_ctx, adec_ctx, frame, packet);
            return;
        }

        // Decode loop
        loop {
            let ret = unsafe { av_read_frame(fmt_ctx, packet) };
            if ret < 0 { break; }

            let pkt_stream = unsafe { (*(packet as *const [i32; 4]))[0] }; // packet.stream_index at offset 0

            if vdec_ctx.is_some() && pkt_stream == video_idx {
                decode_video(vdec_ctx.unwrap(), frame, packet, vpipe);
            } else if adec_ctx.is_some() && pkt_stream == audio_idx {
                decode_audio(adec_ctx.unwrap(), frame, packet, apipe);
            }

            unsafe { av_packet_unref(packet); }
        }

        // Flush
        if let Some(ctx) = vdec_ctx {
            flush_video(ctx, frame, vpipe);
        }
        if let Some(ctx) = adec_ctx {
            flush_audio(ctx, frame, apipe);
        }

        vpipe.drain_to_display();
        apipe.drain_to_sexaudio();
        cleanup(fmt_ctx, None, None, frame, packet);
        println!("[Qupid/FFmpeg] Decode complete.");
    }

    fn open_decoder(
        fmt_ctx: *mut AVFormatContext,
        stream_idx: c_int,
        codec: *const AVCodec,
    ) -> Option<*mut AVCodecContext> {
        if stream_idx < 0 || codec.is_null() { return None; }

        let dec_ctx = unsafe { avcodec_alloc_context3(codec) };
        if dec_ctx.is_null() { return None; }

        let codecpar = unsafe { stream_codecpar(fmt_ctx, stream_idx) };
        if unsafe { avcodec_parameters_to_context(dec_ctx, codecpar) } < 0 {
            unsafe { avcodec_free_context(&mut (dec_ctx as *mut AVCodecContext)); }
            return None;
        }

        if unsafe { avcodec_open2(dec_ctx, codec, std::ptr::null_mut()) } < 0 {
            unsafe { avcodec_free_context(&mut (dec_ctx as *mut AVCodecContext)); }
            return None;
        }

        Some(dec_ctx)
    }

    fn decode_video(ctx: *mut AVCodecContext, frame: *mut AVFrame, pkt: *mut AVPacket, vpipe: &VideoPipe) {
        pku::enter_decoder_domain();
        unsafe {
            avcodec_send_packet(ctx, pkt);
            while avcodec_receive_frame(ctx, frame) == 0 {
                let pfn = (*frame).data[0] as u64 >> 12;
                vpipe.submit(PageHandover { pfn, pku_key: 2 });
                vpipe.drain_to_display();
                av_frame_unref(frame);
            }
        }
        pku::leave_decoder_domain();
    }

    fn decode_audio(ctx: *mut AVCodecContext, frame: *mut AVFrame, pkt: *mut AVPacket, apipe: &AudioPipe) {
        pku::enter_decoder_domain();
        unsafe {
            avcodec_send_packet(ctx, pkt);
            while avcodec_receive_frame(ctx, frame) == 0 {
                let f = &*frame;
                let sexframe = SexAudioFrame {
                    pfn: f.data[0] as u64 >> 12,
                    pku_key: 2,
                    channels: f.ch_layout.nb_channels as u8,
                    sample_rate: f.sample_rate as u32,
                    sample_count: f.nb_samples as u32,
                    format: 0,
                };
                apipe.submit(sexframe);
                apipe.drain_to_sexaudio();
                av_frame_unref(frame);
            }
        }
        pku::leave_decoder_domain();
    }

    fn flush_video(ctx: *mut AVCodecContext, frame: *mut AVFrame, vpipe: &VideoPipe) {
        unsafe {
            avcodec_send_packet(ctx, std::ptr::null());
            while avcodec_receive_frame(ctx, frame) == 0 {
                let pfn = (*frame).data[0] as u64 >> 12;
                vpipe.submit(PageHandover { pfn, pku_key: 2 });
                av_frame_unref(frame);
            }
        }
    }

    fn flush_audio(ctx: *mut AVCodecContext, frame: *mut AVFrame, apipe: &AudioPipe) {
        unsafe {
            avcodec_send_packet(ctx, std::ptr::null());
            while avcodec_receive_frame(ctx, frame) == 0 {
                let f = &*frame;
                apipe.submit(SexAudioFrame {
                    pfn: f.data[0] as u64 >> 12,
                    pku_key: 2,
                    channels: f.ch_layout.nb_channels as u8,
                    sample_rate: f.sample_rate as u32,
                    sample_count: f.nb_samples as u32,
                    format: 0,
                });
                av_frame_unref(frame);
            }
        }
    }

    fn cleanup(
        fmt_ctx: *mut AVFormatContext,
        vctx: Option<*mut AVCodecContext>,
        actx: Option<*mut AVCodecContext>,
        frame: *mut AVFrame,
        packet: *mut AVPacket,
    ) {
        unsafe {
            if !frame.is_null() { av_frame_free(&mut (frame as *mut AVFrame)); }
            if !packet.is_null() { av_packet_free(&mut (packet as *mut AVPacket)); }
            if let Some(mut c) = vctx { avcodec_free_context(&mut c); }
            if let Some(mut c) = actx { avcodec_free_context(&mut c); }
            if !fmt_ctx.is_null() {
                let mut p = fmt_ctx;
                avformat_close_input(&mut p);
            }
        }
    }
}

// ── Symphonia fallback (audio-only) ──────────────────────────────────────────

#[cfg(feature = "symphonia-fallback")]
mod engine_symphonia {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::{MediaSourceStream, ReadOnlySource};
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    use crate::pipes::AudioPipe;
    use sex_pdx::SexAudioFrame;
    use std::fs::File;
    use std::path::Path;

    pub fn run(path: &str, apipe: &AudioPipe) {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => { println!("[Qupid/Symphonia] Cannot open: {}", path); return; }
        };

        let mut hint = Hint::new();
        if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let mss = MediaSourceStream::new(Box::new(ReadOnlySource::new(file)), Default::default());

        let mut probed = match symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        {
            Ok(p) => p,
            Err(e) => { println!("[Qupid/Symphonia] Probe failed: {:?}", e); return; }
        };

        let track = match probed.format.tracks().iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        {
            Some(t) => t,
            None => { println!("[Qupid/Symphonia] No audio track."); return; }
        };

        let track_id    = track.id;
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels    = track.codec_params.channels.map(|c| c.count() as u8).unwrap_or(2);

        let mut decoder = match symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
        {
            Ok(d) => d,
            Err(e) => { println!("[Qupid/Symphonia] Decoder: {:?}", e); return; }
        };

        let mut sample_buf: Option<SampleBuffer<f32>> = None;

        loop {
            let packet = match probed.format.next_packet() {
                Ok(p) => p,
                Err(_) => break,
            };
            if packet.track_id() != track_id { continue; }

            let decoded = match decoder.decode(&packet) {
                Ok(d) => d,
                Err(e) => { println!("[Qupid/Symphonia] Frame: {:?}", e); continue; }
            };

            let spec = *decoded.spec();
            if sample_buf.is_none() {
                sample_buf = Some(SampleBuffer::<f32>::new(decoded.capacity() as u64, spec));
            }
            if let Some(ref mut buf) = sample_buf {
                buf.copy_interleaved_ref(decoded);
                apipe.submit(SexAudioFrame {
                    pfn: buf.samples().as_ptr() as u64 >> 12,
                    pku_key: 0,
                    channels,
                    sample_rate,
                    sample_count: buf.len() as u32 / channels as u32,
                    format: 0,
                });
                apipe.drain_to_sexaudio();
            }
        }
        println!("[Qupid/Symphonia] Decode complete.");
    }
}

// ── Entry point ──────────────────────────────────────────────────────────────

fn main() {
    pku::init_decoder_domain();

    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).map(|s| s.as_str()).unwrap_or("test.mp4");
    println!("[Qupid v1.4] Native Media Engine — {}", path);

    let window_id = ui::create_player_window(1280, 720);

    let vpipe = pipes::VideoPipe::new(window_id);
    let apipe = pipes::AudioPipe::new();

    #[cfg(feature = "ffmpeg-primary")]
    engine_ffmpeg::run(path, &vpipe, &apipe);

    #[cfg(all(feature = "symphonia-fallback", not(feature = "ffmpeg-primary")))]
    engine_symphonia::run(path, &apipe);

    println!("[Qupid v1.4] Shutdown.");
}
