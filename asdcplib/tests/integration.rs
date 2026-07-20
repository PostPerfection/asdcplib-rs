/// Helpers shared by the roundtrip tests.
mod util {
    use std::path::PathBuf;

    /// Unique temp path so concurrent test threads never collide.
    pub fn temp_path(tag: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "asdcplib-{tag}-{}-{unique}.mxf",
            std::process::id()
        ))
    }

    /// Synthetic JPEG 2000 codestream: real SOC/SIZ/SOD/EOC markers wrapped
    /// around filler. asdcplib stores frames opaquely, so this is enough to
    /// prove the bytes survive a write/read cycle.
    pub fn synthetic_j2c(seed: u8, len: usize) -> Vec<u8> {
        assert!(len > 8, "need room for the markers");
        let mut data = vec![0xff, 0x4f, 0xff, 0x51]; // SOC, SIZ
        data.extend((0..len - 8).map(|i| seed.wrapping_add(i as u8)));
        data.extend([0xff, 0x93, 0xff, 0xd9]); // SOD, EOC
        data
    }
}

#[cfg(test)]
mod tests {
    use asdcplib::*;

    #[test]
    fn test_version() {
        let v = version();
        assert!(!v.is_empty(), "version string should not be empty");
        // asdcplib versions look like "2.12.X" or similar
        assert!(v.contains('.'), "version string should contain a dot: {v}");
    }

    #[test]
    fn test_essence_type_nonexistent_file() {
        let result = essence_type("/nonexistent/file.mxf");
        // Should return an error for a file that doesn't exist
        assert!(result.is_err());
    }

    /// Upstream RawEssenceType skips detection when the path is not a file and
    /// leaves the result OK, so a missing file reports Ok(Unknown) rather than
    /// an error. Unlike essence_type, which does fail.
    #[test]
    fn test_raw_essence_type_nonexistent_file() {
        let t = raw_essence_type("/nonexistent/file.j2c")
            .expect("raw_essence_type reports OK for a missing path");
        assert_eq!(t, EssenceType::Unknown);
        assert!(essence_type("/nonexistent/file.mxf").is_err());
    }

    #[test]
    fn test_rational() {
        let r = Rational::new(24, 1);
        assert_eq!(r.numerator, 24);
        assert_eq!(r.denominator, 1);
        assert!((r.quotient() - 24.0).abs() < f64::EPSILON);

        let r2 = Rational::new(24000, 1001);
        let expected = 24000.0 / 1001.0;
        assert!((r2.quotient() - expected).abs() < 0.0001);
    }

    #[test]
    fn test_edit_rate_constants() {
        assert_eq!(EDIT_RATE_24, Rational::new(24, 1));
        assert_eq!(EDIT_RATE_25, Rational::new(25, 1));
        assert_eq!(EDIT_RATE_48, Rational::new(48, 1));
        assert_eq!(SAMPLE_RATE_48K, Rational::new(48000, 1));
    }

    #[test]
    fn test_writer_info_default() {
        let info = WriterInfo::default();
        assert!(!info.encrypted_essence);
        assert!(!info.uses_hmac);
        assert_eq!(info.label_set, LabelSet::Smpte);
        assert_eq!(info.asset_uuid, [0u8; 16]);
    }

    #[test]
    fn test_writer_info_fields() {
        let info = WriterInfo {
            asset_uuid: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            encrypted_essence: true,
            uses_hmac: true,
            label_set: LabelSet::Interop,
            ..Default::default()
        };

        assert_eq!(info.asset_uuid[0], 1);
        assert!(info.encrypted_essence);
        assert!(info.uses_hmac);
        assert_eq!(info.label_set, LabelSet::Interop);
    }

    #[test]
    fn test_essence_type_from_i32() {
        assert_eq!(EssenceType::from(0), EssenceType::Unknown);
        assert_eq!(EssenceType::from(2), EssenceType::Jpeg2000);
        assert_eq!(EssenceType::from(3), EssenceType::Pcm24b48k);
        assert_eq!(EssenceType::from(5), EssenceType::TimedText);
        assert_eq!(EssenceType::from(6), EssenceType::Jpeg2000Stereo);
        assert_eq!(EssenceType::from(8), EssenceType::DcDataDolbyAtmos);
        assert_eq!(EssenceType::from(999), EssenceType::Unknown);
    }

    /// Numbering must track ASDCP::EssenceType_t in AS_DCP.h. Before this was
    /// mapped, every AS-02/IMF file came back as Unknown.
    #[test]
    fn test_essence_type_as02_variants() {
        assert_eq!(EssenceType::from(9), EssenceType::As02Jpeg2000);
        assert_eq!(EssenceType::from(10), EssenceType::As02Pcm24b48k);
        assert_eq!(EssenceType::from(11), EssenceType::As02Pcm24b96k);
        assert_eq!(EssenceType::from(12), EssenceType::As02TimedText);
        assert_eq!(EssenceType::from(13), EssenceType::As02Isxd);
        assert_eq!(EssenceType::from(14), EssenceType::As02Aces);
        assert_eq!(EssenceType::from(15), EssenceType::As02Iab);
        assert_eq!(EssenceType::from(16), EssenceType::As02JpegXs);
        assert_eq!(EssenceType::from(17), EssenceType::JpegXs);
        // ESS_MAX and beyond stay Unknown
        assert_eq!(EssenceType::from(18), EssenceType::Unknown);
    }

    /// Guards against the mapping silently collapsing onto one variant.
    #[test]
    fn test_essence_type_mapping_is_injective() {
        let mapped: Vec<EssenceType> = (1..=17).map(EssenceType::from).collect();
        for (i, a) in mapped.iter().enumerate() {
            assert_ne!(*a, EssenceType::Unknown, "code {} maps to Unknown", i + 1);
            for b in &mapped[i + 1..] {
                assert_ne!(a, b, "two distinct codes map to {a:?}");
            }
        }
    }
}

#[cfg(test)]
mod crypto_tests {
    use asdcplib::LabelSet;
    use asdcplib::crypto::*;

    #[test]
    fn test_aes_enc_context_init() {
        let mut ctx = AesEncContext::new();
        let key = [0u8; 16];
        let result = ctx.init_key(&key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_aes_dec_context_init() {
        let mut ctx = AesDecContext::new();
        let key = [0u8; 16];
        let result = ctx.init_key(&key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hmac_context_init() {
        let mut ctx = HmacContext::new();
        let key = [0u8; 16];
        let result = ctx.init_key(&key, LabelSet::Smpte);
        assert!(result.is_ok());
    }

    #[test]
    fn test_aes_enc_set_ivec() {
        let mut ctx = AesEncContext::new();
        let key = [0u8; 16];
        ctx.init_key(&key).unwrap();
        let ivec = [1u8; 16];
        let result = ctx.set_ivec(&ivec);
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod jp2k_tests {
    use asdcplib::WriterInfo;
    use asdcplib::jp2k::*;

    #[test]
    fn test_jp2k_reader_open_nonexistent() {
        let mut reader = MxfReader::new();
        let result = reader.open_read("/nonexistent/picture.mxf");
        assert!(result.is_err());
    }

    #[test]
    fn test_jp2k_writer_create() {
        // Just ensure it doesn't panic
        let _writer = MxfWriter::new();
    }

    #[test]
    fn test_stereo_writer_create() {
        let _writer = StereoMxfWriter::new();
    }

    #[test]
    fn test_picture_descriptor_fields() {
        let desc = PictureDescriptor {
            edit_rate: asdcplib::Rational::new(24, 1),
            sample_rate: asdcplib::Rational::new(24, 1),
            stored_width: 2048,
            stored_height: 1080,
            aspect_ratio: asdcplib::Rational::new(1998, 1080),
            container_duration: 24 * 60, // 1 minute
            component_count: 3,
        };
        assert_eq!(desc.stored_width, 2048);
        assert_eq!(desc.stored_height, 1080);
        assert_eq!(desc.component_count, 3);
    }

    fn descriptor(frames: u32) -> PictureDescriptor {
        PictureDescriptor {
            edit_rate: asdcplib::EDIT_RATE_24,
            sample_rate: asdcplib::EDIT_RATE_24,
            stored_width: 2048,
            stored_height: 1080,
            aspect_ratio: asdcplib::Rational::new(1998, 1080),
            container_duration: frames,
            component_count: 3,
        }
    }

    #[test]
    fn test_jp2k_roundtrip() {
        let path = crate::util::temp_path("jp2k-roundtrip");
        let path_string = path.to_string_lossy().to_string();
        let info = WriterInfo {
            product_uuid: [7; 16],
            asset_uuid: [8; 16],
            ..Default::default()
        };
        let frames: Vec<Vec<u8>> = (0..3)
            .map(|i| crate::util::synthetic_j2c(i as u8 * 40 + 1, 4096 + i * 32))
            .collect();

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(
                    &path_string,
                    &info,
                    &descriptor(frames.len() as u32),
                    16_384,
                )
                .unwrap();
            for frame in &frames {
                writer.write_frame(frame, None, None).unwrap();
            }
            writer.finalize().unwrap();
        }

        assert_eq!(
            asdcplib::essence_type(&path_string).unwrap(),
            asdcplib::EssenceType::Jpeg2000
        );

        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();

            let desc = reader.picture_descriptor().unwrap();
            assert_eq!(desc.stored_width, 2048);
            assert_eq!(desc.stored_height, 1080);
            assert_eq!(desc.edit_rate, asdcplib::EDIT_RATE_24);
            assert_eq!(desc.container_duration, frames.len() as u32);

            let read_info = reader.writer_info().unwrap();
            assert_eq!(read_info.asset_uuid, [8; 16]);

            // every frame must come back byte-identical, at its own length
            for (i, expected) in frames.iter().enumerate() {
                let mut buf = vec![0u8; 8192];
                let size = reader.read_frame(i as u32, &mut buf, None, None).unwrap();
                assert_eq!(size, expected.len(), "frame {i} length");
                assert_eq!(&buf[..size], expected.as_slice(), "frame {i} bytes");
            }
            reader.close().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }

    /// Encrypt with AES + HMAC on write, decrypt on read, and prove the
    /// plaintext survives byte-exact. Also proves a wrong or missing key
    /// cannot recover it: the check value and the HMAC both reject a bad key.
    #[test]
    fn test_jp2k_encrypted_roundtrip() {
        use asdcplib::LabelSet;
        use asdcplib::crypto::{AesDecContext, AesEncContext, HmacContext};

        let path = crate::util::temp_path("jp2k-encrypted-roundtrip");
        let path_string = path.to_string_lossy().to_string();

        let key = [0x2b; 16];
        let ivec = [0x9c; 16];
        let info = WriterInfo {
            asset_uuid: [8; 16],
            context_id: [0xc7; 16],
            cryptographic_key_id: [0xd4; 16],
            encrypted_essence: true,
            uses_hmac: true,
            ..Default::default()
        };
        // distinct payloads and lengths so a frame mix-up cannot pass
        let frames: Vec<Vec<u8>> = (0..3)
            .map(|i| crate::util::synthetic_j2c(i as u8 * 40 + 1, 4096 + i * 32))
            .collect();

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(&path_string, &info, &descriptor(frames.len() as u32), 16_384)
                .unwrap();
            let mut enc = AesEncContext::new();
            enc.init_key(&key).unwrap();
            enc.set_ivec(&ivec).unwrap();
            let mut hmac = HmacContext::new();
            hmac.init_key(&key, LabelSet::Smpte).unwrap();
            for frame in &frames {
                writer
                    .write_frame(frame, Some(&mut enc), Some(&mut hmac))
                    .unwrap();
            }
            writer.finalize().unwrap();
        }

        // header advertises the essence as encrypted + integrity-protected
        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();
            let read_info = reader.writer_info().unwrap();
            assert!(read_info.encrypted_essence);
            assert!(read_info.uses_hmac);
            reader.close().unwrap();
        }

        // correct key + hmac: every frame comes back byte-identical
        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();
            let mut dec = AesDecContext::new();
            dec.init_key(&key).unwrap();
            let mut hmac = HmacContext::new();
            hmac.init_key(&key, LabelSet::Smpte).unwrap();
            for (i, expected) in frames.iter().enumerate() {
                let mut buf = vec![0u8; 8192];
                let size = reader
                    .read_frame(i as u32, &mut buf, Some(&mut dec), Some(&mut hmac))
                    .unwrap();
                assert_eq!(size, expected.len(), "frame {i} length");
                assert_eq!(&buf[..size], expected.as_slice(), "frame {i} bytes");
            }
            reader.close().unwrap();
        }

        // wrong decryption key: the encrypted check value rejects it
        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();
            let mut dec = AesDecContext::new();
            dec.init_key(&[0xff; 16]).unwrap();
            let mut hmac = HmacContext::new();
            hmac.init_key(&key, LabelSet::Smpte).unwrap();
            let mut buf = vec![0u8; 8192];
            assert!(
                reader
                    .read_frame(0, &mut buf, Some(&mut dec), Some(&mut hmac))
                    .is_err()
            );
            reader.close().unwrap();
        }

        // right decryption key but wrong hmac key: integrity check rejects it
        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();
            let mut dec = AesDecContext::new();
            dec.init_key(&key).unwrap();
            let mut hmac = HmacContext::new();
            hmac.init_key(&[0xff; 16], LabelSet::Smpte).unwrap();
            let mut buf = vec![0u8; 8192];
            assert!(
                reader
                    .read_frame(0, &mut buf, Some(&mut dec), Some(&mut hmac))
                    .is_err()
            );
            reader.close().unwrap();
        }

        // no key at all: the read returns ciphertext, never the plaintext
        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();
            let mut buf = vec![0u8; 8192];
            let size = reader.read_frame(0, &mut buf, None, None).unwrap();
            assert_ne!(
                &buf[..size],
                frames[0].as_slice(),
                "ciphertext must not equal plaintext"
            );
            reader.close().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_jp2k_stereo_roundtrip() {
        let path = crate::util::temp_path("jp2k-stereo-roundtrip");
        let path_string = path.to_string_lossy().to_string();
        let info = WriterInfo {
            asset_uuid: [9; 16],
            ..Default::default()
        };
        // distinct payloads so a left/right mix-up cannot pass
        let left = crate::util::synthetic_j2c(0x11, 3072);
        let right = crate::util::synthetic_j2c(0xa0, 2048);

        {
            let mut writer = StereoMxfWriter::new();
            writer
                .open_write(&path_string, &info, &descriptor(1), 16_384)
                .unwrap();
            writer
                .write_frame(&left, StereoscopicPhase::Left, None, None)
                .unwrap();
            writer
                .write_frame(&right, StereoscopicPhase::Right, None, None)
                .unwrap();
            writer.finalize().unwrap();
        }

        assert_eq!(
            asdcplib::essence_type(&path_string).unwrap(),
            asdcplib::EssenceType::Jpeg2000Stereo
        );

        {
            let mut reader = StereoMxfReader::new();
            reader.open_read(&path_string).unwrap();

            let desc = reader.picture_descriptor().unwrap();
            assert_eq!(desc.stored_width, 2048);
            assert_eq!(desc.stored_height, 1080);

            let mut buf = vec![0u8; 8192];
            let size = reader
                .read_frame(0, StereoscopicPhase::Left, &mut buf, None, None)
                .unwrap();
            assert_eq!(&buf[..size], left.as_slice(), "left eye");

            let size = reader
                .read_frame(0, StereoscopicPhase::Right, &mut buf, None, None)
                .unwrap();
            assert_eq!(&buf[..size], right.as_slice(), "right eye");
            reader.close().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod pcm_tests {
    use asdcplib::pcm::*;
    use asdcplib::{Rational, WriterInfo};

    #[test]
    fn test_pcm_reader_open_nonexistent() {
        let mut reader = MxfReader::new();
        let result = reader.open_read("/nonexistent/audio.mxf");
        assert!(result.is_err());
    }

    #[test]
    fn test_channel_format_values() {
        assert_eq!(ChannelFormat::None as i32, 0);
        assert_eq!(ChannelFormat::Cfg1 as i32, 1);
        assert_eq!(ChannelFormat::Cfg3 as i32, 3);
    }

    #[test]
    fn test_pcm_roundtrip() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "asdcplib-pcm-roundtrip-{}-{unique}.mxf",
            std::process::id()
        ));
        let path_string = path.to_string_lossy().to_string();
        let descriptor = AudioDescriptor {
            edit_rate: Rational::new(24, 1),
            audio_sampling_rate: Rational::new(48_000, 1),
            locked: true,
            channel_count: 6,
            quantization_bits: 24,
            block_align: 18,
            avg_bps: 864_000,
            linked_track_id: 0,
            container_duration: 1,
            channel_format: ChannelFormat::Cfg1,
        };
        let info = WriterInfo {
            product_uuid: [1; 16],
            asset_uuid: [2; 16],
            context_id: [3; 16],
            ..Default::default()
        };
        let frame = vec![0x5a; 36_000];

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(&path_string, &info, &descriptor, 16_384)
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();
            let actual_descriptor = reader.audio_descriptor().unwrap();
            assert_eq!(actual_descriptor.channel_count, 6);
            assert_eq!(
                actual_descriptor.audio_sampling_rate,
                Rational::new(48_000, 1)
            );
            let mut output = vec![0; frame.len()];
            let size = reader.read_frame(0, &mut output, None, None).unwrap();
            assert_eq!(size, frame.len());
            assert_eq!(&output[..size], frame.as_slice());
            reader.close().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod timed_text_tests {
    use asdcplib::timed_text::*;
    use asdcplib::{EDIT_RATE_24, Error, WriterInfo};

    #[test]
    fn test_timed_text_reader_open_nonexistent() {
        let mut reader = MxfReader::new();
        let result = reader.open_read("/nonexistent/subtitle.mxf");
        assert!(result.is_err());
    }

    const SUBTITLE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<SubtitleReel xmlns="http://www.smpte-ra.org/schemas/428-7/2007/DCST">
  <Id>urn:uuid:11111111-2222-3333-4444-555555555555</Id>
  <ReelNumber>1</ReelNumber>
  <EditRate>24 1</EditRate>
  <TimeCodeRate>24</TimeCodeRate>
  <SubtitleList>
    <Font ID="theFont">
      <Subtitle SpotNumber="1" TimeIn="00:00:00:000" TimeOut="00:00:04:000">
        <Text>hello</Text>
      </Subtitle>
    </Font>
  </SubtitleList>
</SubtitleReel>"#;

    /// Write a subtitle MXF and hand back its path.
    fn write_subtitle_mxf(tag: &str) -> std::path::PathBuf {
        let path = crate::util::temp_path(tag);
        let path_string = path.to_string_lossy().to_string();
        let info = WriterInfo {
            asset_uuid: [4; 16],
            ..Default::default()
        };
        let desc = TimedTextDescriptor {
            edit_rate: EDIT_RATE_24,
            container_duration: 96,
            asset_id: [5; 16],
        };
        let mut writer = MxfWriter::new();
        writer
            .open_write(&path_string, &info, &desc, 16_384)
            .unwrap();
        writer
            .write_timed_text_resource(SUBTITLE_XML, None, None)
            .unwrap();
        writer.finalize().unwrap();
        path
    }

    #[test]
    fn test_timed_text_roundtrip() {
        let path = write_subtitle_mxf("timed-text-roundtrip");
        let path_string = path.to_string_lossy().to_string();

        assert_eq!(
            asdcplib::essence_type(&path_string).unwrap(),
            asdcplib::EssenceType::TimedText
        );

        let mut reader = MxfReader::new();
        reader.open_read(&path_string).unwrap();

        let desc = reader.descriptor().unwrap();
        assert_eq!(desc.edit_rate, EDIT_RATE_24);
        assert_eq!(desc.container_duration, 96);

        let read_info = reader.writer_info().unwrap();
        assert_eq!(read_info.asset_uuid, [4; 16]);

        let mut buf = vec![0u8; 64 * 1024];
        let size = reader
            .read_timed_text_resource(&mut buf, None, None)
            .unwrap();
        assert_eq!(size, SUBTITLE_XML.len());
        assert_eq!(&buf[..size], SUBTITLE_XML.as_bytes());
        reader.close().unwrap();

        std::fs::remove_file(path).unwrap();
    }

    /// A short buffer used to be silently truncated and reported as success.
    #[test]
    fn test_timed_text_buffer_too_small() {
        let path = write_subtitle_mxf("timed-text-smallbuf");
        let path_string = path.to_string_lossy().to_string();

        let mut reader = MxfReader::new();
        reader.open_read(&path_string).unwrap();

        let capacity = 16;
        let mut buf = vec![0u8; capacity];
        let err = reader
            .read_timed_text_resource(&mut buf, None, None)
            .expect_err("truncated read must not report success");

        match err {
            Error::BufferTooSmall {
                needed,
                capacity: c,
            } => {
                assert_eq!(needed, SUBTITLE_XML.len());
                assert_eq!(c, capacity);
            }
            other => panic!("expected BufferTooSmall, got {other:?}"),
        }
        // the buffer is left untouched rather than holding a partial document
        assert_eq!(buf, vec![0u8; capacity]);

        // an adequate buffer still works on the same reader
        let mut big = vec![0u8; 64 * 1024];
        let size = reader
            .read_timed_text_resource(&mut big, None, None)
            .unwrap();
        assert_eq!(&big[..size], SUBTITLE_XML.as_bytes());
        reader.close().unwrap();

        std::fs::remove_file(path).unwrap();
    }

    /// Exactly-fitting buffer is not truncation.
    #[test]
    fn test_timed_text_exact_buffer_ok() {
        let path = write_subtitle_mxf("timed-text-exact");
        let path_string = path.to_string_lossy().to_string();

        let mut reader = MxfReader::new();
        reader.open_read(&path_string).unwrap();
        let mut buf = vec![0u8; SUBTITLE_XML.len()];
        let size = reader
            .read_timed_text_resource(&mut buf, None, None)
            .unwrap();
        assert_eq!(size, SUBTITLE_XML.len());
        assert_eq!(buf, SUBTITLE_XML.as_bytes());
        reader.close().unwrap();

        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod atmos_tests {
    use asdcplib::atmos::*;
    use asdcplib::{EDIT_RATE_24, WriterInfo};

    #[test]
    fn test_atmos_reader_open_nonexistent() {
        let mut reader = MxfReader::new();
        let result = reader.open_read("/nonexistent/atmos.mxf");
        assert!(result.is_err());
    }

    #[test]
    fn test_atmos_roundtrip() {
        let path = crate::util::temp_path("atmos-roundtrip");
        let path_string = path.to_string_lossy().to_string();
        let info = WriterInfo {
            asset_uuid: [6; 16],
            ..Default::default()
        };
        let desc = AtmosDescriptor {
            edit_rate: EDIT_RATE_24,
            container_duration: 2,
            asset_id: [0xa1; 16],
            data_essence_coding: [0; 16],
            first_frame: 0,
            max_channel_count: 10,
            max_object_count: 118,
            atmos_id: [0xb2; 16],
            atmos_version: 1,
        };
        // atmos frames are opaque bytestreams, so distinct filler is enough
        let frames: Vec<Vec<u8>> = (0..2)
            .map(|i| (0..2048).map(|b| (b as u8).wrapping_add(i * 7)).collect())
            .collect();

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(&path_string, &info, &desc, 16_384)
                .unwrap();
            for frame in &frames {
                writer.write_frame(frame, None, None).unwrap();
            }
            writer.finalize().unwrap();
        }

        assert_eq!(
            asdcplib::essence_type(&path_string).unwrap(),
            asdcplib::EssenceType::DcDataDolbyAtmos
        );

        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();

            let actual = reader.atmos_descriptor().unwrap();
            assert_eq!(actual.edit_rate, EDIT_RATE_24);
            assert_eq!(actual.max_channel_count, 10);
            assert_eq!(actual.max_object_count, 118);
            assert_eq!(actual.atmos_version, 1);
            assert_eq!(actual.atmos_id, [0xb2; 16]);
            assert_eq!(actual.container_duration, frames.len() as u32);

            for (i, expected) in frames.iter().enumerate() {
                let mut buf = vec![0u8; 4096];
                let size = reader.read_frame(i as u32, &mut buf, None, None).unwrap();
                assert_eq!(size, expected.len(), "frame {i} length");
                assert_eq!(&buf[..size], expected.as_slice(), "frame {i} bytes");
            }
            reader.close().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod as02_jp2k_tests {
    use asdcplib::WriterInfo;
    use asdcplib::as02::jp2k::*;
    use asdcplib::jp2k::PictureDescriptor;

    fn descriptor(frames: u32) -> PictureDescriptor {
        PictureDescriptor {
            edit_rate: asdcplib::EDIT_RATE_24,
            sample_rate: asdcplib::EDIT_RATE_24,
            stored_width: 2048,
            stored_height: 1080,
            aspect_ratio: asdcplib::Rational::new(1998, 1080),
            container_duration: frames,
            component_count: 3,
        }
    }

    #[test]
    fn test_as02_jp2k_reader_open_nonexistent() {
        let mut reader = MxfReader::new();
        assert!(reader.open_read("/nonexistent/as02.mxf").is_err());
    }

    #[test]
    fn test_as02_jp2k_roundtrip() {
        let path = crate::util::temp_path("as02-jp2k-roundtrip");
        let path_string = path.to_string_lossy().to_string();
        let info = WriterInfo {
            product_uuid: [7; 16],
            asset_uuid: [8; 16],
            ..Default::default()
        };
        // distinct payloads and lengths so a frame mix-up cannot pass
        let frames: Vec<Vec<u8>> = (0..3)
            .map(|i| crate::util::synthetic_j2c(i as u8 * 40 + 1, 4096 + i * 32))
            .collect();

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(
                    &path_string,
                    &info,
                    &descriptor(frames.len() as u32),
                    16_384,
                )
                .unwrap();
            for frame in &frames {
                writer.write_frame(frame, None, None).unwrap();
            }
            writer.finalize().unwrap();
        }

        assert_eq!(
            asdcplib::essence_type(&path_string).unwrap(),
            asdcplib::EssenceType::As02Jpeg2000
        );

        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();

            let desc = reader.picture_descriptor().unwrap();
            assert_eq!(desc.stored_width, 2048);
            assert_eq!(desc.stored_height, 1080);
            assert_eq!(desc.edit_rate, asdcplib::EDIT_RATE_24);
            assert_eq!(desc.container_duration, frames.len() as u32);

            let read_info = reader.writer_info().unwrap();
            assert_eq!(read_info.asset_uuid, [8; 16]);

            for (i, expected) in frames.iter().enumerate() {
                let mut buf = vec![0u8; 8192];
                let size = reader.read_frame(i as u32, &mut buf, None, None).unwrap();
                assert_eq!(size, expected.len(), "frame {i} length");
                assert_eq!(&buf[..size], expected.as_slice(), "frame {i} bytes");
            }
            reader.close().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod as02_pcm_tests {
    use asdcplib::as02::pcm::*;
    use asdcplib::pcm::{AudioDescriptor, ChannelFormat};
    use asdcplib::{Rational, WriterInfo};

    #[test]
    fn test_as02_pcm_reader_open_nonexistent() {
        let mut reader = MxfReader::new();
        assert!(
            reader
                .open_read("/nonexistent/as02.mxf", Rational::new(24, 1))
                .is_err()
        );
    }

    #[test]
    fn test_as02_pcm_roundtrip() {
        let path = crate::util::temp_path("as02-pcm-roundtrip");
        let path_string = path.to_string_lossy().to_string();
        // block_align 18 = 24-bit * 6ch; 2000 samples/frame at 48k/24fps -> 36000 bytes
        let descriptor = AudioDescriptor {
            edit_rate: Rational::new(24, 1),
            audio_sampling_rate: Rational::new(48_000, 1),
            locked: true,
            channel_count: 6,
            quantization_bits: 24,
            block_align: 18,
            avg_bps: 864_000,
            linked_track_id: 0,
            container_duration: 0,
            channel_format: ChannelFormat::Cfg1,
        };
        let info = WriterInfo {
            asset_uuid: [2; 16],
            ..Default::default()
        };
        // two distinct clip-wrapped frames so ordering is verified
        let frames: Vec<Vec<u8>> = (0..2)
            .map(|i| {
                (0..36_000)
                    .map(|b| (b as u8).wrapping_add(i * 91))
                    .collect()
            })
            .collect();

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(&path_string, &info, &descriptor, 16_384)
                .unwrap();
            for frame in &frames {
                writer.write_frame(frame, None, None).unwrap();
            }
            writer.finalize().unwrap();
        }

        assert_eq!(
            asdcplib::essence_type(&path_string).unwrap(),
            asdcplib::EssenceType::As02Pcm24b48k
        );

        {
            let mut reader = MxfReader::new();
            reader
                .open_read(&path_string, Rational::new(24, 1))
                .unwrap();

            let actual = reader.audio_descriptor().unwrap();
            assert_eq!(actual.channel_count, 6);
            assert_eq!(actual.quantization_bits, 24);
            assert_eq!(actual.block_align, 18);
            assert_eq!(actual.audio_sampling_rate, Rational::new(48_000, 1));

            for (i, expected) in frames.iter().enumerate() {
                let mut buf = vec![0u8; 36_000];
                let size = reader.read_frame(i as u32, &mut buf, None, None).unwrap();
                assert_eq!(size, expected.len(), "frame {i} length");
                assert_eq!(&buf[..size], expected.as_slice(), "frame {i} bytes");
            }
            reader.close().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod as02_timed_text_tests {
    use asdcplib::as02::timed_text::*;
    use asdcplib::timed_text::TimedTextDescriptor;
    use asdcplib::{EDIT_RATE_24, WriterInfo};

    // minimal ST 2067-2 (IMSC1 / TTML) subtitle document
    const SUBTITLE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<tt xmlns="http://www.w3.org/ns/ttml" xml:lang="en">
  <body>
    <div>
      <p begin="00:00:00.000" end="00:00:04.000">hello imf</p>
    </div>
  </body>
</tt>"#;

    #[test]
    fn test_as02_timed_text_reader_open_nonexistent() {
        let mut reader = MxfReader::new();
        assert!(reader.open_read("/nonexistent/as02.mxf").is_err());
    }

    #[test]
    fn test_as02_timed_text_roundtrip() {
        let path = crate::util::temp_path("as02-timed-text-roundtrip");
        let path_string = path.to_string_lossy().to_string();
        let info = WriterInfo {
            asset_uuid: [4; 16],
            ..Default::default()
        };
        let desc = TimedTextDescriptor {
            edit_rate: EDIT_RATE_24,
            container_duration: 96,
            asset_id: [5; 16],
        };

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(&path_string, &info, &desc, 16_384)
                .unwrap();
            writer
                .write_timed_text_resource(SUBTITLE_XML, None, None)
                .unwrap();
            writer.finalize().unwrap();
        }

        assert_eq!(
            asdcplib::essence_type(&path_string).unwrap(),
            asdcplib::EssenceType::As02TimedText
        );

        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();

            let desc = reader.descriptor().unwrap();
            assert_eq!(desc.edit_rate, EDIT_RATE_24);

            let read_info = reader.writer_info().unwrap();
            assert_eq!(read_info.asset_uuid, [4; 16]);

            let mut buf = vec![0u8; 64 * 1024];
            let size = reader
                .read_timed_text_resource(&mut buf, None, None)
                .unwrap();
            assert_eq!(size, SUBTITLE_XML.len());
            assert_eq!(&buf[..size], SUBTITLE_XML.as_bytes());
            reader.close().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }
}
