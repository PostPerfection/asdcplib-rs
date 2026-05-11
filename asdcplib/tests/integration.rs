#[cfg(test)]
mod tests {
    use asdcplib::*;

    #[test]
    fn test_version() {
        let v = version();
        assert!(!v.is_empty(), "version string should not be empty");
        // asdcplib versions look like "2.12.X" or similar
        assert!(
            v.contains('.'),
            "version string should contain a dot: {v}"
        );
    }

    #[test]
    fn test_essence_type_nonexistent_file() {
        let result = essence_type("/nonexistent/file.mxf");
        // Should return an error for a file that doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_raw_essence_type_nonexistent_file() {
        let result = raw_essence_type("/nonexistent/file.j2c");
        // raw_essence_type returns Ok(Unknown) for files it can't identify
        match result {
            Ok(t) => assert_eq!(t, EssenceType::Unknown),
            Err(_) => {} // also acceptable
        }
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
        let mut info = WriterInfo::default();
        info.asset_uuid = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        info.encrypted_essence = true;
        info.uses_hmac = true;
        info.label_set = LabelSet::Interop;

        assert_eq!(info.asset_uuid[0], 1);
        assert_eq!(info.encrypted_essence, true);
        assert_eq!(info.uses_hmac, true);
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
}

#[cfg(test)]
mod crypto_tests {
    use asdcplib::crypto::*;
    use asdcplib::LabelSet;

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
}

#[cfg(test)]
mod pcm_tests {
    use asdcplib::pcm::*;

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
}

#[cfg(test)]
mod timed_text_tests {
    use asdcplib::timed_text::*;

    #[test]
    fn test_timed_text_reader_open_nonexistent() {
        let mut reader = MxfReader::new();
        let result = reader.open_read("/nonexistent/subtitle.mxf");
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod atmos_tests {
    use asdcplib::atmos::*;

    #[test]
    fn test_atmos_reader_open_nonexistent() {
        let mut reader = MxfReader::new();
        let result = reader.open_read("/nonexistent/atmos.mxf");
        assert!(result.is_err());
    }
}
