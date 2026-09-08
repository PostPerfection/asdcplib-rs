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

    pub const CINEMA_2K_FIXTURE: &str = "cinema2k_64x64.j2c";
    pub const IMF_4K_FIXTURE: &str = "imf4k_black_3840x2160.j2c";

    /// A real JPEG 2000 codestream from `tests/fixtures`. Building a
    /// `PictureDescriptor` needs one of these: the synthetic stub below carries
    /// no SIZ payload, COD or QCD for the parser to read.
    pub fn fixture(name: &str) -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
    }

    /// Synthetic JPEG 2000 codestream: real SOC/SIZ/SOD/EOC markers wrapped
    /// around filler. asdcplib stores frames opaquely, so this is enough to
    /// prove the bytes survive a write/read cycle, and giving every frame its
    /// own seed and length catches a frame mix-up.
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

    #[test]
    fn picture_essence_coding_follows_the_rsize() {
        use asdcplib::jp2k::{
            CodestreamHeader, PICTURE_ESSENCE_CODING_BROADCAST_PROFILE_1,
            PICTURE_ESSENCE_CODING_CINEMA_2K, PICTURE_ESSENCE_CODING_CINEMA_4K,
            PICTURE_ESSENCE_CODING_IMF_2K_LOSSY, PICTURE_ESSENCE_CODING_IMF_4K_LOSSY_6_3,
            PICTURE_ESSENCE_CODING_IMF_8K_LOSSY, PictureDescriptor,
            picture_essence_coding_for_rsize,
        };

        assert_eq!(
            picture_essence_coding_for_rsize(0x0536),
            PICTURE_ESSENCE_CODING_IMF_4K_LOSSY_6_3
        );
        assert_eq!(
            picture_essence_coding_for_rsize(0x0003),
            PICTURE_ESSENCE_CODING_CINEMA_2K
        );
        assert_eq!(
            picture_essence_coding_for_rsize(0x0004),
            PICTURE_ESSENCE_CODING_CINEMA_4K
        );
        assert_eq!(
            picture_essence_coding_for_rsize(0x0417),
            PICTURE_ESSENCE_CODING_IMF_2K_LOSSY
        );
        assert_eq!(
            picture_essence_coding_for_rsize(0x0611),
            PICTURE_ESSENCE_CODING_IMF_8K_LOSSY
        );
        assert_eq!(
            picture_essence_coding_for_rsize(0x0000),
            PICTURE_ESSENCE_CODING_BROADCAST_PROFILE_1
        );

        let frame = crate::util::fixture(crate::util::IMF_4K_FIXTURE);
        let codestream = CodestreamHeader::parse(&frame).unwrap();
        let descriptor = PictureDescriptor {
            edit_rate: EDIT_RATE_24,
            sample_rate: EDIT_RATE_24,
            stored_width: codestream.xsize,
            stored_height: codestream.ysize,
            aspect_ratio: Rational::new(codestream.xsize as i32, codestream.ysize as i32),
            container_duration: 1,
            codestream,
        };

        let path = crate::util::temp_path("picture-essence-coding-for-rsize");
        let path_string = path.to_string_lossy().to_string();
        let info = WriterInfo::default();

        {
            let mut writer = as02::jp2k::MxfWriter::new();
            writer
                .open_write(&path_string, &info, &descriptor, 16_384)
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        let mut reader = as02::jp2k::MxfReader::new();
        reader.open_read(&path_string).unwrap();
        assert_eq!(
            reader.rgba_descriptor().unwrap().picture_essence_coding,
            Some(picture_essence_coding_for_rsize(
                descriptor.codestream.rsize
            ))
        );
        reader.close().unwrap();

        std::fs::remove_file(path).unwrap();
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

    /// The cinema fixture's SIZ, COD and QCD values, read straight off the file.
    #[test]
    fn test_codestream_header_parse() {
        let codestream =
            CodestreamHeader::parse(&crate::util::fixture(crate::util::CINEMA_2K_FIXTURE)).unwrap();

        assert_eq!(codestream.rsize, 0x0003);
        assert_eq!(codestream.xsize, 64);
        assert_eq!(codestream.ysize, 64);
        assert_eq!(codestream.x_osize, 0);
        assert_eq!(codestream.y_osize, 0);
        assert_eq!(codestream.xt_size, 64);
        assert_eq!(codestream.yt_size, 64);
        assert_eq!(codestream.components.len(), 3);
        for component in &codestream.components {
            assert_eq!(component.bit_depth(), 12);
            assert!(!component.is_signed());
            assert_eq!(component.x_rsize, 1);
            assert_eq!(component.y_rsize, 1);
        }
        assert!(codestream.quantization_default.spqcd.len() > 1);
    }

    /// Nothing but a codestream builds a `CodestreamHeader`, so filler cannot
    /// reach the sub-descriptor.
    #[test]
    fn test_codestream_header_rejects_non_codestream() {
        assert!(CodestreamHeader::parse(&crate::util::synthetic_j2c(0x11, 4096)).is_err());
    }

    #[test]
    fn test_picture_descriptor_fields() {
        let desc = descriptor(24 * 60);
        assert_eq!(desc.stored_width, 64);
        assert_eq!(desc.stored_height, 64);
        assert_eq!(desc.codestream.components.len(), 3);
        assert_eq!(desc.codestream.rsize, 0x0003);
    }

    fn descriptor(frames: u32) -> PictureDescriptor {
        let codestream =
            CodestreamHeader::parse(&crate::util::fixture(crate::util::CINEMA_2K_FIXTURE)).unwrap();
        PictureDescriptor {
            edit_rate: asdcplib::EDIT_RATE_24,
            sample_rate: asdcplib::EDIT_RATE_24,
            stored_width: codestream.xsize,
            stored_height: codestream.ysize,
            aspect_ratio: asdcplib::Rational::new(1, 1),
            container_duration: frames,
            codestream,
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
            assert_eq!(desc.stored_width, 64);
            assert_eq!(desc.stored_height, 64);
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

    /// The JPEG2000PictureSubDescriptor must describe the codestream that was
    /// wrapped. It used to be written all zeros, so asdcp-info reported
    /// Xsize 0, Ysize 0 and an empty CodingStyleDefault.
    #[test]
    fn test_jp2k_sub_descriptor_from_codestream() {
        let path = crate::util::temp_path("jp2k-sub-descriptor");
        let path_string = path.to_string_lossy().to_string();
        let frame = crate::util::fixture(crate::util::CINEMA_2K_FIXTURE);
        let written = descriptor(1);

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(&path_string, &WriterInfo::default(), &written, 16_384)
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        let mut reader = MxfReader::new();
        reader.open_read(&path_string).unwrap();
        let read = reader.picture_descriptor().unwrap();

        assert_eq!(read.codestream.rsize, 0x0003);
        assert_eq!(read.codestream.xsize, 64);
        assert_eq!(read.codestream.ysize, 64);
        assert_eq!(read.codestream.components.len(), 3);
        for component in &read.codestream.components {
            assert_eq!(component.bit_depth(), 12);
            assert_eq!(component.x_rsize, 1);
            assert_eq!(component.y_rsize, 1);
        }
        assert_ne!(read.codestream.coding_style_default.decomposition_levels, 0);
        // the whole header, COD and QCD included, survives the roundtrip
        assert_eq!(read.codestream, written.codestream);

        reader.close().unwrap();
        std::fs::remove_file(path).unwrap();
    }

    /// Write a JP2K MXF with the ST 2084 (PQ) TransferCharacteristic UL set on
    /// the picture descriptor, then read it back and confirm the UL survived.
    #[test]
    fn test_jp2k_transfer_characteristic_roundtrip() {
        let path = crate::util::temp_path("jp2k-transfer-characteristic");
        let path_string = path.to_string_lossy().to_string();
        let info = WriterInfo::default();
        let frames: Vec<Vec<u8>> = (0..2)
            .map(|i| crate::util::synthetic_j2c(i as u8 * 40 + 1, 4096 + i * 32))
            .collect();

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write_transfer(
                    &path_string,
                    &info,
                    &descriptor(frames.len() as u32),
                    &TRANSFER_CHARACTERISTIC_ST2084,
                    16_384,
                )
                .unwrap();
            for frame in &frames {
                writer.write_frame(frame, None, None).unwrap();
            }
            writer.finalize().unwrap();
        }

        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();
            assert_eq!(
                reader.transfer_characteristic().unwrap(),
                Some(TRANSFER_CHARACTERISTIC_ST2084),
                "ST 2084 transfer characteristic UL must round-trip"
            );
            reader.close().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }

    /// A plain writer sets no TransferCharacteristic, so the reader reports None.
    #[test]
    fn test_jp2k_transfer_characteristic_absent() {
        let path = crate::util::temp_path("jp2k-transfer-characteristic-absent");
        let path_string = path.to_string_lossy().to_string();
        let info = WriterInfo::default();
        let frame = crate::util::synthetic_j2c(0x33, 4096);

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(&path_string, &info, &descriptor(1), 16_384)
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();
            assert_eq!(reader.transfer_characteristic().unwrap(), None);
            reader.close().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }

    /// Full HDR block (ST 2084 transfer, P3D65 primaries, ST 2086 mastering
    /// display) on the AS-DCP JP2K writer round-trips through the reader.
    #[test]
    fn test_jp2k_hdr_roundtrip() {
        let path = crate::util::temp_path("jp2k-hdr-roundtrip");
        let path_string = path.to_string_lossy().to_string();
        let info = WriterInfo::default();
        let frames: Vec<Vec<u8>> = (0..2)
            .map(|i| crate::util::synthetic_j2c(i as u8 * 40 + 1, 4096 + i * 32))
            .collect();

        let hdr = HdrMetadata {
            transfer_characteristic: Some(TRANSFER_CHARACTERISTIC_ST2084),
            color_primaries: Some(COLOR_PRIMARIES_P3D65),
            mastering_display_primaries: Some([[34000, 16000], [13250, 34500], [7500, 3000]]),
            mastering_display_white_point: Some([15635, 16450]),
            mastering_display_max_luminance: Some(48_0000),
            mastering_display_min_luminance: Some(50),
        };

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write_hdr(
                    &path_string,
                    &info,
                    &descriptor(frames.len() as u32),
                    &hdr,
                    16_384,
                )
                .unwrap();
            for frame in &frames {
                writer.write_frame(frame, None, None).unwrap();
            }
            writer.finalize().unwrap();
        }

        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();
            assert_eq!(reader.hdr_metadata().unwrap(), hdr);
            // the single-field accessor agrees with the full block
            assert_eq!(
                reader.transfer_characteristic().unwrap(),
                Some(TRANSFER_CHARACTERISTIC_ST2084)
            );
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
                .open_write(
                    &path_string,
                    &info,
                    &descriptor(frames.len() as u32),
                    16_384,
                )
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
            assert_eq!(desc.stored_width, 64);
            assert_eq!(desc.stored_height, 64);

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

    /// Wrap a 5.1 PCM MXF with SMPTE 377-4 MCA labels and read them back: six
    /// channel labels, one soundfield group, and the MCA ChannelAssignment UL.
    #[test]
    fn test_pcm_mca_labels_roundtrip() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "asdcplib-pcm-mca-{}-{unique}.mxf",
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
            channel_format: ChannelFormat::Cfg6, // MCA
        };
        let info = WriterInfo {
            asset_uuid: [3; 16],
            ..Default::default()
        };
        let frame = vec![0x5a; 36_000];

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write_mca(
                    &path_string,
                    &info,
                    &descriptor,
                    "51(L,R,C,LFE,Ls,Rs)",
                    None,
                    16_384,
                )
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();
            let mca = reader.mca_labels().unwrap();
            assert_eq!(mca.channel_labels, 6, "one label per 5.1 channel");
            assert_eq!(mca.soundfield_groups, 1, "one 5.1 soundfield group");
            assert!(
                mca.has_mca_channel_assignment,
                "WaveAudioDescriptor must carry the MCA ChannelAssignment UL"
            );
            reader.close().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }

    fn mca_temp_path(tag: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "asdcplib-pcm-mca-{tag}-{}-{unique}.mxf",
            std::process::id()
        ))
    }

    fn mca_descriptor(channel_count: u32) -> AudioDescriptor {
        let block_align = channel_count * 3;
        AudioDescriptor {
            edit_rate: Rational::new(24, 1),
            audio_sampling_rate: Rational::new(48_000, 1),
            locked: true,
            channel_count,
            quantization_bits: 24,
            block_align,
            avg_bps: block_align * 48_000,
            linked_track_id: 0,
            container_duration: 1,
            channel_format: ChannelFormat::Cfg6,
        }
    }

    /// Write one frame of a labelled PCM MXF, then read every MCA label
    /// subdescriptor back out of it.
    fn write_and_read_mca_labels(
        path: &std::path::Path,
        channel_count: u32,
        mca_config: &str,
        mca_language: Option<&str>,
    ) -> Vec<McaLabelSubDescriptor> {
        let path_string = path.to_string_lossy().to_string();
        let descriptor = mca_descriptor(channel_count);
        let info = WriterInfo {
            asset_uuid: [4; 16],
            ..Default::default()
        };
        // one 24fps frame at 48kHz is 2000 samples
        let frame = vec![0x5a; (descriptor.block_align * 2_000) as usize];

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write_mca(
                    &path_string,
                    &info,
                    &descriptor,
                    mca_config,
                    mca_language,
                    16_384,
                )
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        let mut reader = MxfReader::new();
        reader.open_read(&path_string).unwrap();
        let labels = reader.mca_label_subdescriptors().unwrap();
        reader.close().unwrap();
        labels
    }

    /// A 5.1 wrap must read back the soundfield group followed by its six
    /// channels, each carrying its tag symbol, one-based channel id and a link
    /// back to the group.
    #[test]
    fn test_pcm_mca_label_subdescriptors_roundtrip() {
        let path = mca_temp_path("symbols");
        let labels = write_and_read_mca_labels(&path, 6, "51(L,R,C,LFE,Ls,Rs)", None);

        let symbols: Vec<&str> = labels.iter().map(|l| l.tag_symbol.as_str()).collect();
        assert_eq!(
            symbols,
            ["sg51", "chL", "chR", "chC", "chLFE", "chLs", "chRs"],
            "tag symbols must come back in the order they were written"
        );

        let group = &labels[0];
        assert_eq!(group.kind, McaLabelKind::SoundfieldGroup);
        assert_eq!(group.tag_name.as_deref(), Some("5.1"));
        assert_eq!(group.channel_id, None, "a soundfield group has no channel");
        assert_eq!(group.soundfield_group_link_id, None);

        for (offset, channel) in labels[1..].iter().enumerate() {
            assert_eq!(channel.kind, McaLabelKind::AudioChannel);
            assert_eq!(
                channel.channel_id,
                Some(offset as u32 + 1),
                "channel ids are one-based and in wrap order"
            );
            assert_eq!(
                channel.soundfield_group_link_id,
                Some(group.link_id),
                "every 5.1 channel links back to the soundfield group"
            );
            assert_eq!(channel.spoken_language.as_deref(), Some("en-US"));
        }
        assert_eq!(labels[1].tag_name.as_deref(), Some("Left"));
        assert_eq!(labels[4].tag_name.as_deref(), Some("LFE"));
        assert_ne!(
            labels[1].label_dictionary_id, labels[2].label_dictionary_id,
            "Left and Right must not share a label UL"
        );

        std::fs::remove_file(path).unwrap();
    }

    /// The accessibility channels must be distinguishable from each other and
    /// from the main programme: hearing impaired, visually impaired narrative
    /// and sign language video each keep their own tag symbol and name.
    #[test]
    fn test_pcm_mca_accessibility_labels_roundtrip() {
        let path = mca_temp_path("access");
        let labels = write_and_read_mca_labels(&path, 9, "51(L,R,C,LFE,Ls,Rs),HI,VIN,SLVS", None);

        let symbols: Vec<&str> = labels.iter().map(|l| l.tag_symbol.as_str()).collect();
        assert_eq!(
            symbols,
            [
                "sg51", "chL", "chR", "chC", "chLFE", "chLs", "chRs", "chHI", "chVIN", "SLVS"
            ]
        );

        let accessibility = &labels[7..];
        let names: Vec<Option<&str>> = accessibility
            .iter()
            .map(|l| l.tag_name.as_deref())
            .collect();
        assert_eq!(
            names,
            [
                Some("Hearing Impaired"),
                Some("Visually Impaired-Narrative"),
                Some("Sign Language Video Stream")
            ]
        );
        for label in accessibility {
            assert_eq!(label.kind, McaLabelKind::AudioChannel);
            assert_eq!(
                label.soundfield_group_link_id, None,
                "accessibility channels sit outside the 5.1 soundfield group"
            );
        }
        assert_eq!(accessibility[0].channel_id, Some(7));
        assert_eq!(accessibility[2].channel_id, Some(9));

        std::fs::remove_file(path).unwrap();
    }

    /// The requested RFC 5646 tag must land on the soundfield group and every
    /// channel label.
    #[test]
    fn test_pcm_mca_spoken_language_roundtrip() {
        let path = mca_temp_path("language");
        let labels = write_and_read_mca_labels(&path, 6, "51(L,R,C,LFE,Ls,Rs)", Some("fr-CA"));

        assert_eq!(labels.len(), 7);
        for label in &labels {
            assert_eq!(
                label.spoken_language.as_deref(),
                Some("fr-CA"),
                "{} must carry the requested language",
                label.tag_symbol
            );
        }

        std::fs::remove_file(path).unwrap();
    }

    /// Without a language, asdcplib's own default must come back.
    #[test]
    fn test_pcm_mca_default_spoken_language_is_en_us() {
        let path = mca_temp_path("default-language");
        let labels = write_and_read_mca_labels(&path, 6, "51(L,R,C,LFE,Ls,Rs)", None);

        assert_eq!(labels.len(), 7);
        for label in &labels {
            assert_eq!(
                label.spoken_language.as_deref(),
                Some("en-US"),
                "{} must carry the default language",
                label.tag_symbol
            );
        }

        std::fs::remove_file(path).unwrap();
    }

    /// A PCM MXF wrapped without MCA labels reports no subdescriptors.
    #[test]
    fn test_pcm_without_mca_labels_has_no_subdescriptors() {
        let path = mca_temp_path("none");
        let path_string = path.to_string_lossy().to_string();
        let mut descriptor = mca_descriptor(6);
        descriptor.channel_format = ChannelFormat::Cfg1;
        let info = WriterInfo::default();
        let frame = vec![0x5a; 36_000];

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(&path_string, &info, &descriptor, 16_384)
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        let mut reader = MxfReader::new();
        reader.open_read(&path_string).unwrap();
        assert!(reader.mca_label_subdescriptors().unwrap().is_empty());
        reader.close().unwrap();

        std::fs::remove_file(path).unwrap();
    }

    /// A label count that disagrees with the channel count must fail the wrap.
    #[test]
    fn test_pcm_mca_channel_count_mismatch_fails() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "asdcplib-pcm-mca-bad-{}-{unique}.mxf",
            std::process::id()
        ));
        let path_string = path.to_string_lossy().to_string();
        let descriptor = AudioDescriptor {
            edit_rate: Rational::new(24, 1),
            audio_sampling_rate: Rational::new(48_000, 1),
            locked: true,
            channel_count: 2,
            quantization_bits: 24,
            block_align: 6,
            avg_bps: 288_000,
            linked_track_id: 0,
            container_duration: 1,
            channel_format: ChannelFormat::Cfg6,
        };
        let info = WriterInfo::default();
        let mut writer = MxfWriter::new();
        // six MCA labels against a two-channel descriptor
        assert!(
            writer
                .open_write_mca(
                    &path_string,
                    &info,
                    &descriptor,
                    "51(L,R,C,LFE,Ls,Rs)",
                    None,
                    16_384
                )
                .is_err()
        );
        let _ = std::fs::remove_file(path);
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

    /// Write a subtitle MXF with an embedded font as an ancillary resource, then
    /// read it back: the reader must enumerate one resource with the matching
    /// UUID and OpenType MIME, and return byte-identical font data.
    #[test]
    fn test_ancillary_resource_roundtrip() {
        let path = crate::util::temp_path("timed-text-ancillary");
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
        // stand-in font payload; the bytes only need to survive the round trip
        let font: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
        let font_uuid = [0xAB; 16];

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write_with_resources(
                    &path_string,
                    &info,
                    &desc,
                    &[AncillaryResourceInfo {
                        uuid: font_uuid,
                        mime_type: MimeType::OpenType,
                    }],
                    32_768,
                )
                .unwrap();
            writer
                .write_timed_text_resource(SUBTITLE_XML, None, None)
                .unwrap();
            writer
                .write_ancillary_resource(
                    &font,
                    &font_uuid,
                    "application/x-font-opentype",
                    None,
                    None,
                )
                .unwrap();
            writer.finalize().unwrap();
        }

        let mut reader = MxfReader::new();
        reader.open_read(&path_string).unwrap();
        assert_eq!(reader.ancillary_resource_count().unwrap(), 1);
        let info0 = reader.ancillary_resource_info(0).unwrap();
        assert_eq!(info0.uuid, font_uuid);
        assert_eq!(info0.mime_type, MimeType::OpenType);

        let mut buf = vec![0u8; 64 * 1024];
        let n = reader
            .read_ancillary_resource(&font_uuid, &mut buf, None, None)
            .unwrap();
        assert_eq!(n, font.len(), "font byte count must survive the round trip");
        assert_eq!(&buf[..n], font.as_slice(), "font bytes must be identical");
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
    use asdcplib::jp2k::{
        COLOR_PRIMARIES_BT2020, CodestreamHeader, HdrMetadata, PICTURE_ESSENCE_CODING_CINEMA_2K,
        PICTURE_ESSENCE_CODING_IMF_4K_LOSSY_6_3, PictureDescriptor, TRANSFER_CHARACTERISTIC_ST2084,
    };

    /// RGBAValue_RGB_12: what a 12-bit RGB codestream produces for both
    /// PixelLayout and J2CLayout.
    const RGB_12_LAYOUT: [u8; 16] = [b'R', 12, b'G', 12, b'B', 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    /// SMPTE 377-1 FrameLayout for a progressive frame.
    const FRAME_LAYOUT_FULL_FRAME: u8 = 0;

    /// SMPTE 377-1 ScanningDirection for left to right, top to bottom.
    const SCANNING_DIRECTION_LEFT_TO_RIGHT_TOP_TO_BOTTOM: u8 = 0;

    fn descriptor_for(fixture_name: &str, frames: u32) -> PictureDescriptor {
        let codestream = CodestreamHeader::parse(&crate::util::fixture(fixture_name)).unwrap();
        PictureDescriptor {
            edit_rate: asdcplib::EDIT_RATE_24,
            sample_rate: asdcplib::EDIT_RATE_24,
            stored_width: codestream.xsize,
            stored_height: codestream.ysize,
            aspect_ratio: asdcplib::Rational::new(codestream.xsize as i32, codestream.ysize as i32),
            container_duration: frames,
            codestream,
        }
    }

    fn descriptor(frames: u32) -> PictureDescriptor {
        descriptor_for(crate::util::IMF_4K_FIXTURE, frames)
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
            assert_eq!(desc.stored_width, 3840);
            assert_eq!(desc.stored_height, 2160);
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

    /// Wrap a fixture and read back both the sub-descriptor and the RGBA
    /// essence descriptor the AS-02 writer derived from its codestream.
    fn assert_as02_descriptors(
        fixture_name: &str,
        expected_picture_essence_coding: [u8; 16],
        expected_rsize: u16,
        expected_size: (u32, u32),
    ) {
        let path = crate::util::temp_path("as02-jp2k-descriptors");
        let path_string = path.to_string_lossy().to_string();
        let frame = crate::util::fixture(fixture_name);
        let written = descriptor_for(fixture_name, 1);

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(&path_string, &WriterInfo::default(), &written, 16_384)
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        let mut reader = MxfReader::new();
        reader.open_read(&path_string).unwrap();

        let read = reader.picture_descriptor().unwrap();
        assert_eq!(read.codestream.rsize, expected_rsize);
        assert_eq!(read.codestream.xsize, expected_size.0);
        assert_eq!(read.codestream.ysize, expected_size.1);
        assert_eq!(read.codestream.components.len(), 3);
        for component in &read.codestream.components {
            assert_eq!(component.bit_depth(), 12);
            assert_eq!(component.x_rsize, 1);
            assert_eq!(component.y_rsize, 1);
        }
        assert_eq!(read.codestream, written.codestream);

        let rgba = reader.rgba_descriptor().unwrap();
        assert_eq!(
            rgba.picture_essence_coding,
            Some(expected_picture_essence_coding)
        );
        // 12-bit RGB, so RGBAValue_RGB_12
        assert_eq!(
            rgba.pixel_layout,
            [b'R', 12, b'G', 12, b'B', 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(rgba.component_max_ref, Some(4095));
        assert_eq!(rgba.component_min_ref, Some(0));

        reader.close().unwrap();
        std::fs::remove_file(path).unwrap();
    }

    /// An IMF 4K lossy codestream at main level 6 sub level 3 (Rsiz 0x0536) must
    /// land as that level's label, not the Broadcast Profile 1 label the writer
    /// used to hardcode. The expected UL is the PictureCompression Netflix's Sol
    /// Levante App 2E picture carries.
    #[test]
    fn test_as02_jp2k_descriptors_imf_4k() {
        assert_eq!(
            PICTURE_ESSENCE_CODING_IMF_4K_LOSSY_6_3,
            [
                0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x0d, 0x04, 0x01, 0x02, 0x02, 0x03, 0x01,
                0x03, 0x12
            ]
        );
        assert_as02_descriptors(
            crate::util::IMF_4K_FIXTURE,
            PICTURE_ESSENCE_CODING_IMF_4K_LOSSY_6_3,
            0x0536,
            (3840, 2160),
        );
    }

    /// A DCI cinema codestream (Rsiz 0x0003) maps to the 2K cinema label.
    #[test]
    fn test_as02_jp2k_descriptors_cinema_2k() {
        assert_as02_descriptors(
            crate::util::CINEMA_2K_FIXTURE,
            PICTURE_ESSENCE_CODING_CINEMA_2K,
            0x0003,
            (64, 64),
        );
    }

    /// Write an AS-02 JP2K MXF with ST 2084 transfer, BT.2020 primaries and a
    /// full ST 2086 mastering display block, then read it all back and assert
    /// every field round-trips. This is the ST 2067-21 HDR essence path.
    #[test]
    fn test_as02_jp2k_hdr_roundtrip() {
        let path = crate::util::temp_path("as02-jp2k-hdr-roundtrip");
        let path_string = path.to_string_lossy().to_string();
        let info = WriterInfo::default();
        let frames: Vec<Vec<u8>> = (0..2)
            .map(|i| crate::util::synthetic_j2c(i as u8 * 40 + 1, 4096 + i * 32))
            .collect();

        // ST 2086 BT.2020 display primaries and D65 white point in 0.00002 units,
        // 1000 cd/m^2 max and 0.0001 cd/m^2 min in 0.0001 units.
        let hdr = HdrMetadata {
            transfer_characteristic: Some(TRANSFER_CHARACTERISTIC_ST2084),
            color_primaries: Some(COLOR_PRIMARIES_BT2020),
            mastering_display_primaries: Some([[35400, 14600], [8500, 39850], [6550, 2300]]),
            mastering_display_white_point: Some([15635, 16450]),
            mastering_display_max_luminance: Some(10_000_000),
            mastering_display_min_luminance: Some(1),
        };

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write_hdr(
                    &path_string,
                    &info,
                    &descriptor(frames.len() as u32),
                    &hdr,
                    16_384,
                )
                .unwrap();
            for frame in &frames {
                writer.write_frame(frame, None, None).unwrap();
            }
            writer.finalize().unwrap();
        }

        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();
            assert_eq!(reader.hdr_metadata().unwrap(), hdr);
            reader.close().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }

    /// Every RGBA descriptor and JPEG 2000 sub-descriptor item an IMF CPL
    /// EssenceDescriptorList has to repeat, read back off a two-frame 12-bit RGB
    /// file. Photon compares the CPL entry against these values verbatim.
    #[test]
    fn test_as02_jp2k_full_descriptors_roundtrip() {
        let path = crate::util::temp_path("as02-jp2k-full-descriptors");
        let path_string = path.to_string_lossy().to_string();
        let frame = crate::util::fixture(crate::util::IMF_4K_FIXTURE);
        let written = descriptor(2);

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(&path_string, &WriterInfo::default(), &written, 16_384)
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        let mut reader = MxfReader::new();
        reader.open_read(&path_string).unwrap();
        let rgba = reader.rgba_essence_descriptor().unwrap();
        let sub = reader.jpeg2000_sub_descriptor().unwrap();
        reader.close().unwrap();
        std::fs::remove_file(path).unwrap();

        assert_ne!(rgba.instance_id, [0u8; 16]);
        assert_ne!(sub.instance_id, [0u8; 16]);
        assert_eq!(rgba.sub_descriptors, vec![sub.instance_id]);
        assert_eq!(rgba.locators, Vec::<[u8; 16]>::new());
        assert_eq!(rgba.generation_id, None);
        assert_eq!(sub.generation_id, None);

        assert_eq!(rgba.linked_track_id, Some(1));
        assert_eq!(rgba.sample_rate, asdcplib::EDIT_RATE_24);
        assert_eq!(rgba.container_duration, Some(2));
        assert_ne!(rgba.essence_container, [0u8; 16]);
        assert_eq!(rgba.codec, None);

        assert_eq!(rgba.frame_layout, FRAME_LAYOUT_FULL_FRAME);
        assert_eq!(rgba.stored_width, 3840);
        assert_eq!(rgba.stored_height, 2160);
        assert_eq!(rgba.aspect_ratio, asdcplib::Rational::new(3840, 2160));
        assert_eq!(
            rgba.picture_essence_coding,
            PICTURE_ESSENCE_CODING_IMF_4K_LOSSY_6_3
        );
        assert_eq!(
            rgba.scanning_direction,
            Some(SCANNING_DIRECTION_LEFT_TO_RIGHT_TOP_TO_BOTTOM)
        );
        assert_eq!(rgba.video_line_map, Some([1, 0]));
        assert_eq!(rgba.pixel_layout, RGB_12_LAYOUT);
        assert_eq!(rgba.component_min_ref, Some(0));
        assert_eq!(rgba.component_max_ref, Some(4095));
        assert_eq!(rgba.alpha_min_ref, None);
        assert_eq!(rgba.alpha_max_ref, None);
        assert_eq!(rgba.hdr, HdrMetadata::default());
        assert_eq!(rgba.coding_equations, None);
        assert_eq!(rgba.alternative_center_cuts, Vec::<[u8; 16]>::new());

        // the sub-descriptor repeats the codestream's SIZ grid
        let codestream = &written.codestream;
        assert_eq!(sub.rsize, codestream.rsize);
        assert_eq!(sub.xsize, codestream.xsize);
        assert_eq!(sub.ysize, codestream.ysize);
        assert_eq!(sub.x_osize, codestream.x_osize);
        assert_eq!(sub.y_osize, codestream.y_osize);
        assert_eq!(sub.xt_size, codestream.xt_size);
        assert_eq!(sub.yt_size, codestream.yt_size);
        assert_eq!(sub.xt_osize, codestream.xt_osize);
        assert_eq!(sub.yt_osize, codestream.yt_osize);
        assert_eq!(sub.csize, codestream.components.len() as u16);

        // count and item size, then Ssize, XRsize, YRsize per component
        let mut expected_sizing = vec![0, 0, 0, 3, 0, 0, 0, 3];
        for component in &codestream.components {
            expected_sizing.extend([component.ssize, component.x_rsize, component.y_rsize]);
        }
        assert_eq!(sub.picture_component_sizing, Some(expected_sizing));

        let coding_style = sub.coding_style_default.as_ref().unwrap();
        assert_eq!(coding_style[0], codestream.coding_style_default.scod);
        assert_eq!(
            coding_style[1],
            codestream.coding_style_default.progression_order
        );
        let quantization = sub.quantization_default.as_ref().unwrap();
        assert_eq!(quantization[0], codestream.quantization_default.sqcd);
        assert_eq!(
            &quantization[1..],
            codestream.quantization_default.spqcd.as_slice()
        );

        // 3 components at 12 bits, so the same layout the descriptor carries
        assert_eq!(sub.j2c_layout, Some(RGB_12_LAYOUT));
        assert_eq!(
            sub.extended_capabilities,
            codestream.extended_capabilities.clone()
        );
    }

    /// A 12-bit RGB codestream reads back a J2CLayout of three components at 12
    /// bits, which is where Photon takes PixelBitDepth from.
    #[test]
    fn test_as02_jp2k_j2c_layout_follows_component_precision() {
        let path = crate::util::temp_path("as02-jp2k-j2c-layout");
        let path_string = path.to_string_lossy().to_string();
        let frame = crate::util::fixture(crate::util::CINEMA_2K_FIXTURE);
        let written = descriptor_for(crate::util::CINEMA_2K_FIXTURE, 1);

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(&path_string, &WriterInfo::default(), &written, 16_384)
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        let mut reader = MxfReader::new();
        reader.open_read(&path_string).unwrap();
        let sub = reader.jpeg2000_sub_descriptor().unwrap();
        reader.close().unwrap();
        std::fs::remove_file(path).unwrap();

        let layout = sub.j2c_layout.unwrap();
        for (index, component) in written.codestream.components.iter().enumerate() {
            assert_eq!(layout[index * 2], RGB_12_LAYOUT[index * 2]);
            assert_eq!(layout[index * 2 + 1], component.bit_depth());
        }
        assert_eq!(
            &layout[written.codestream.components.len() * 2..],
            &[0u8; 10]
        );
    }

    /// A plain AS-02 writer sets no HDR metadata, so every field reads back None.
    #[test]
    fn test_as02_jp2k_hdr_absent() {
        let path = crate::util::temp_path("as02-jp2k-hdr-absent");
        let path_string = path.to_string_lossy().to_string();
        let info = WriterInfo::default();
        let frame = crate::util::synthetic_j2c(0x55, 4096);

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(&path_string, &info, &descriptor(1), 16_384)
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        {
            let mut reader = MxfReader::new();
            reader.open_read(&path_string).unwrap();
            assert_eq!(reader.hdr_metadata().unwrap(), HdrMetadata::default());
            reader.close().unwrap();
        }

        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod as02_pcm_tests {
    use asdcplib::as02::pcm::*;
    use asdcplib::pcm::{AudioDescriptor, ChannelFormat, McaLabelKind, McaLabelSubDescriptor};
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

    fn mca_descriptor(channel_count: u32) -> AudioDescriptor {
        let block_align = channel_count * 3;
        AudioDescriptor {
            edit_rate: Rational::new(24, 1),
            audio_sampling_rate: Rational::new(48_000, 1),
            locked: true,
            channel_count,
            quantization_bits: 24,
            block_align,
            avg_bps: block_align * 48_000,
            linked_track_id: 0,
            container_duration: 0,
            channel_format: ChannelFormat::Cfg6,
        }
    }

    /// The four ST 2067-2 5.3.6.5 items the tests write on the soundfield group.
    const SOUNDFIELD_GROUP: SoundfieldGroupProperties = SoundfieldGroupProperties {
        language: "en-US",
        title: "Sol Levante",
        title_version: "Original Version",
        audio_content_kind: "PRM",
        audio_element_kind: "FCMP",
    };

    /// Write one clip-wrapped frame of labelled PCM, then read the
    /// ChannelAssignment and every MCA label subdescriptor back out.
    fn write_and_read_mca(
        tag: &str,
        channel_count: u32,
        mca_config: &str,
        language: &str,
    ) -> (Option<[u8; 16]>, Vec<McaLabelSubDescriptor>) {
        let path = crate::util::temp_path(tag);
        let path_string = path.to_string_lossy().to_string();
        let descriptor = mca_descriptor(channel_count);
        // one 24fps frame at 48kHz is 2000 samples
        let frame = vec![0x5a; (descriptor.block_align * 2_000) as usize];

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write_mca(
                    &path_string,
                    &WriterInfo::default(),
                    &descriptor,
                    mca_config,
                    &SoundfieldGroupProperties {
                        language,
                        ..SOUNDFIELD_GROUP
                    },
                    16_384,
                )
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        let mut reader = MxfReader::new();
        reader
            .open_read(&path_string, Rational::new(24, 1))
            .unwrap();
        let assignment = reader.channel_assignment().unwrap();
        let labels = reader.mca_label_subdescriptors().unwrap();
        reader.close().unwrap();
        std::fs::remove_file(path).unwrap();
        (assignment, labels)
    }

    /// An IMF stereo wrap carries the IMF MCA ChannelAssignment, one soundfield
    /// group with the spoken language, and one channel label per channel linked
    /// back to that group.
    #[test]
    fn test_as02_pcm_mca_stereo_roundtrip() {
        let (assignment, labels) = write_and_read_mca("as02-pcm-mca-stereo", 2, "ST(L,R)", "de-DE");

        assert_eq!(assignment, Some(IMF_CHANNEL_ASSIGNMENT_MCA));
        assert_eq!(labels.len(), 3);

        let group = &labels[0];
        assert_eq!(group.kind, McaLabelKind::SoundfieldGroup);
        assert_eq!(group.tag_symbol, "sgST");
        assert_eq!(group.spoken_language.as_deref(), Some("de-DE"));
        assert_soundfield_group_properties(group);

        let channels = &labels[1..];
        assert_eq!(channels.len(), 2);
        for (index, symbol) in ["chL", "chR"].iter().enumerate() {
            let channel = &channels[index];
            assert_eq!(channel.kind, McaLabelKind::AudioChannel);
            assert_eq!(&channel.tag_symbol, symbol);
            assert_eq!(channel.channel_id, Some(index as u32 + 1));
            assert_eq!(channel.soundfield_group_link_id, Some(group.link_id));
        }
    }

    /// A 5.1 wrap carries six channel labels in channel order, one per channel
    /// of the WaveAudioDescriptor.
    #[test]
    fn test_as02_pcm_mca_51_roundtrip() {
        let (assignment, labels) =
            write_and_read_mca("as02-pcm-mca-51", 6, "51(L,R,C,LFE,Ls,Rs)", "en-US");

        assert_eq!(assignment, Some(IMF_CHANNEL_ASSIGNMENT_MCA));
        assert_eq!(labels.len(), 7);

        let group = &labels[0];
        assert_eq!(group.kind, McaLabelKind::SoundfieldGroup);
        assert_eq!(group.tag_symbol, "sg51");
        assert_eq!(group.spoken_language.as_deref(), Some("en-US"));
        assert_soundfield_group_properties(group);

        let symbols = ["chL", "chR", "chC", "chLFE", "chLs", "chRs"];
        assert_eq!(labels[1..].len(), symbols.len());
        for (index, symbol) in symbols.iter().enumerate() {
            let channel = &labels[index + 1];
            assert_eq!(channel.kind, McaLabelKind::AudioChannel);
            assert_eq!(&channel.tag_symbol, symbol);
            assert_eq!(channel.channel_id, Some(index as u32 + 1));
            assert_eq!(channel.soundfield_group_link_id, Some(group.link_id));
        }
    }

    /// The essence container an AS-02 PCM track file carries, ST 382
    /// clip-wrapped WAVE (asdcplib MDD.cpp `WAVWrappingClip`).
    const WAV_WRAPPING_CLIP: [u8; 16] = [
        0x06, 0x0e, 0x2b, 0x34, 0x04, 0x01, 0x01, 0x01, 0x0d, 0x01, 0x03, 0x01, 0x02, 0x06, 0x02,
        0x00,
    ];

    /// Every WaveAudioDescriptor item an IMF CPL EssenceDescriptorList has to
    /// repeat, read back off a two-frame 5.1 MCA file. Photon compares the CPL
    /// entry against these values verbatim.
    #[test]
    fn test_as02_pcm_full_descriptor_roundtrip() {
        let path = crate::util::temp_path("as02-pcm-full-descriptor");
        let path_string = path.to_string_lossy().to_string();
        let written = mca_descriptor(6);
        // one 24fps frame at 48kHz is 2000 samples
        let frame = vec![0x5a; (written.block_align * 2_000) as usize];

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write_mca(
                    &path_string,
                    &WriterInfo::default(),
                    &written,
                    "51(L,R,C,LFE,Ls,Rs)",
                    &SOUNDFIELD_GROUP,
                    16_384,
                )
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        let mut reader = MxfReader::new();
        reader
            .open_read(&path_string, Rational::new(24, 1))
            .unwrap();
        let wave = reader.wave_audio_descriptor().unwrap();
        let labels = reader.mca_label_subdescriptors().unwrap();
        reader.close().unwrap();
        std::fs::remove_file(path).unwrap();

        assert_ne!(wave.instance_id, [0u8; 16]);
        assert_eq!(wave.generation_id, None);
        assert_eq!(wave.locators, Vec::<[u8; 16]>::new());

        assert_eq!(wave.linked_track_id, Some(1));
        // clip-wrapped PCM counts SampleRate and ContainerDuration in samples
        assert_eq!(wave.sample_rate, written.audio_sampling_rate);
        assert_eq!(wave.container_duration, Some(4_000));
        assert_eq!(wave.essence_container, WAV_WRAPPING_CLIP);
        assert_eq!(wave.codec, None);

        assert_eq!(wave.audio_sampling_rate, written.audio_sampling_rate);
        assert_eq!(wave.locked, written.locked);
        assert_eq!(wave.channel_count, written.channel_count);
        assert_eq!(wave.quantization_bits, written.quantization_bits);
        assert_eq!(wave.block_align, written.block_align as u16);
        assert_eq!(wave.avg_bps, written.avg_bps);
        assert_eq!(wave.channel_assignment, Some(IMF_CHANNEL_ASSIGNMENT_MCA));
        // asdcplib never sets SoundEssenceCoding, so it reads back nil
        assert_eq!(wave.sound_essence_coding, [0u8; 16]);
        assert_eq!(wave.audio_ref_level, None);
        assert_eq!(wave.electro_spatial_formulation, None);
        assert_eq!(wave.dial_norm, None);
        assert_eq!(wave.reference_audio_alignment_level, None);
        assert_eq!(wave.reference_image_edit_rate, None);
        assert_eq!(wave.sequence_offset, None);

        // a CPL names each subdescriptor by the InstanceID the descriptor links
        let instance_ids: Vec<[u8; 16]> = labels.iter().map(|label| label.instance_id).collect();
        assert_eq!(instance_ids.len(), 7);
        assert_eq!(wave.sub_descriptors, instance_ids);
        let mut distinct = instance_ids.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), instance_ids.len());
        assert!(!distinct.contains(&[0u8; 16]));
    }

    /// Photon fails a track file whose soundfield group leaves any of the four
    /// items empty, so all four have to survive the write.
    fn assert_soundfield_group_properties(group: &McaLabelSubDescriptor) {
        assert_eq!(group.title.as_deref(), Some(SOUNDFIELD_GROUP.title));
        assert_eq!(
            group.title_version.as_deref(),
            Some(SOUNDFIELD_GROUP.title_version)
        );
        assert_eq!(
            group.audio_content_kind.as_deref(),
            Some(SOUNDFIELD_GROUP.audio_content_kind)
        );
        assert_eq!(
            group.audio_element_kind.as_deref(),
            Some(SOUNDFIELD_GROUP.audio_element_kind)
        );
    }

    /// The four items go on the soundfield group only, never on a channel label.
    #[test]
    fn test_as02_pcm_mca_channel_labels_carry_no_group_properties() {
        let (_, labels) =
            write_and_read_mca("as02-pcm-mca-channel-properties", 2, "ST(L,R)", "en-US");

        for channel in &labels[1..] {
            assert_eq!(channel.kind, McaLabelKind::AudioChannel);
            assert_eq!(channel.title, None);
            assert_eq!(channel.title_version, None);
            assert_eq!(channel.audio_content_kind, None);
            assert_eq!(channel.audio_element_kind, None);
        }
    }

    /// An empty item is refused before anything is written, since Photon reads
    /// an empty string as missing.
    #[test]
    fn test_as02_pcm_mca_empty_group_property_fails() {
        let path = crate::util::temp_path("as02-pcm-mca-empty-property");
        let path_string = path.to_string_lossy().to_string();
        let mut writer = MxfWriter::new();
        assert!(
            writer
                .open_write_mca(
                    &path_string,
                    &WriterInfo::default(),
                    &mca_descriptor(2),
                    "ST(L,R)",
                    &SoundfieldGroupProperties {
                        audio_element_kind: "",
                        ..SOUNDFIELD_GROUP
                    },
                    16_384,
                )
                .is_err()
        );
        assert!(!path.exists());
    }

    /// A config naming no soundfield group cannot carry the four items, so it is
    /// refused rather than written without them.
    #[test]
    fn test_as02_pcm_mca_without_soundfield_group_fails() {
        let path = crate::util::temp_path("as02-pcm-mca-no-group");
        let path_string = path.to_string_lossy().to_string();
        let mut writer = MxfWriter::new();
        assert!(
            writer
                .open_write_mca(
                    &path_string,
                    &WriterInfo::default(),
                    &mca_descriptor(2),
                    "L,R",
                    &SOUNDFIELD_GROUP,
                    16_384,
                )
                .is_err()
        );
    }

    /// A config naming a different number of channels than the descriptor is
    /// refused rather than written.
    #[test]
    fn test_as02_pcm_mca_channel_count_mismatch_fails() {
        let path = crate::util::temp_path("as02-pcm-mca-mismatch");
        let path_string = path.to_string_lossy().to_string();
        let mut writer = MxfWriter::new();
        assert!(
            writer
                .open_write_mca(
                    &path_string,
                    &WriterInfo::default(),
                    &mca_descriptor(6),
                    "ST(L,R)",
                    &SOUNDFIELD_GROUP,
                    16_384,
                )
                .is_err()
        );
    }

    /// A plain AS-02 PCM wrap carries no MCA labels and never the IMF
    /// ChannelAssignment UL, which is what Photon rejects.
    #[test]
    fn test_as02_pcm_without_mca_labels_lacks_imf_channel_assignment() {
        let path = crate::util::temp_path("as02-pcm-no-mca");
        let path_string = path.to_string_lossy().to_string();
        let descriptor = mca_descriptor(2);
        let frame = vec![0x11; (descriptor.block_align * 2_000) as usize];

        {
            let mut writer = MxfWriter::new();
            writer
                .open_write(&path_string, &WriterInfo::default(), &descriptor, 16_384)
                .unwrap();
            writer.write_frame(&frame, None, None).unwrap();
            writer.finalize().unwrap();
        }

        let mut reader = MxfReader::new();
        reader
            .open_read(&path_string, Rational::new(24, 1))
            .unwrap();
        assert_ne!(
            reader.channel_assignment().unwrap(),
            Some(IMF_CHANNEL_ASSIGNMENT_MCA)
        );
        assert!(reader.mca_label_subdescriptors().unwrap().is_empty());
        reader.close().unwrap();
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
