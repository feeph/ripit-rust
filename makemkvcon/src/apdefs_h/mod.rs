/*
    The enum variants and values in this file are derived from
    `makemkv-oss-1.18.3/makemkvgui/inc/lgpl/apdefs.h`.
*/

// The crate 'enum_primitive' exports the macro enum_from_primitive!{} that
// wraps an enum declaration and automatically adds an implementation of
// num::FromPrimitive (reexported here), to allow conversion from primitive
// integers to the enum.
//
// It provides an alternative to the built-in #[derive(FromPrimitive)],
// which requires the unstable std::num::FromPrimitive and is disabled in
// Rust 1.0.
// -- https://andersk.github.io/enum_primitive-rs/enum_primitive/

use num::FromPrimitive;

// class ContentType(Enum):
//     // combined flags are possible, e.g. BluRay disc = 'BluRay + Aacs'
//     Dvd = 1, // AP_DskFsFlagDvdFilesPresent
//     HdDvd = 2, // AP_DskFsFlagHdvdFilesPresent
//     BluRay = 4, // AP_DskFsFlagBlurayFilesPresent
//     Aacs = 8, // AP_DskFsFlagAacsFilesPresent
//     Bdsvm = 16, // AP_DskFsFlagBdsvmFilesPresent

enum_from_primitive! {
    #[derive(Clone, Debug, PartialEq, serde::Serialize)]
    pub enum DriveStatus {
        EmptyClosed = 0, // AP_DriveStateEmptyClosed
        EmptyOpen = 1, // AP_DriveStateEmptyOpen
        DiscInserted = 2, // AP_DriveStateInserted
        DiscLoading = 3, // AP_DriveStateLoading
        NoDrive = 256, // AP_DriveStateNoDrive
        Unmounting = 257, // AP_DriveStateUnmounting
    }
}

enum_from_primitive! {
    /// item attribute id
    /// 
    /// The values are derived from enum '_AP_ItemAttributeId' in
    /// `makemkv-oss-1.18.3/makemkvgui/inc/lgpl/apdefs.h`.
    #[derive(Clone, Debug, PartialEq, serde::Serialize)]
    // #[derive(Debug, FromPrimitive, PartialEq)]
    // #[enum_parse(derive(SomehowParsable, Debug), repr(C, packed), attr(parse_input = &[u16], parse_fn = somehow_parse))]
    pub enum ItemAttributeCode {
        AppDumpDonePartial = 5004, // APP_DUMP_DONE_PARTIAL
        AppDumpDone = 5005, // APP_DUMP_DONE
        AppInitFailed = 5009, // APP_INIT_FAILED
        AppAskFolderCreate = 5013, // APP_ASK_FOLDER_CREATE
        AppFolderInvalid = 5016, // APP_FOLDER_INVALID
        ProgressAppSaveMkvFreeSpace = 5033, // PROGRESS_APP_SAVE_MKV_FREE_SPACE
        ProtDemoKeyExpired = 5021, // PROT_DEMO_KEY_EXPIRED
        AppKeyTypeInvalid = 5095, // APP_KEYTYPE_INVALID
        AppEvalTimeNever = 5067, // APP_EVAL_TIME_NEVER
        AppBackupFailed = 5069, // APP_BACKUP_FAILED
        AppBackupCompleted = 5070, // APP_BACKUP_COMPLETED
        AppBackupCompletedHashFail = 5079, // APP_BACKUP_COMPLETED_HASHFAIL
        ProfileNameDefault = 5086, // PROFILE_NAME_DEFAULT
        VItemName = 5202, // VITEM_NAME
        VItemTimestamp = 5223, // VITEM_TIMESTAMP
        AppInterfaceTitle = 6000, // APP_IFACE_TITLE
        AppCaptionMsg = 6001, // APP_CAPTION_MSG
        AppAboutBoxTitle = 6002, // APP_ABOUTBOX_TITLE
        AppInterfaceOpenFileTitle = 6003, // APP_IFACE_OPENFILE_TITLE
        AppSettingDialogTitle = 6135, // APP_SETTINGDLG_TITLE
        AppBackupDialogTitle = 6136, // APP_BACKUPDLG_TITLE
        AppInterfaceOpenFileFilterTemplate1 = 6007, // APP_IFACE_OPENFILE_FILTER_TEMPLATE1
        AppInterfaceOpenFileFilterTemplate2 = 6008, // APP_IFACE_OPENFILE_FILTER_TEMPLATE2
        AppInterfaceOpenFolderTitle = 6005, // APP_IFACE_OPENFOLDER_TITLE
        AppInterfaceOpenFolderInfoTitle = 6006, // APP_IFACE_OPENFOLDER_INFO_TITLE
        AppInterfaceProgressTitle = 6038, // APP_IFACE_PROGRESS_TITLE
        AppInterfaceProgressElapsedOnly = 6039, // APP_IFACE_PROGRESS_ELAPSED_ONLY
        AppInterfaceProgressElapsedEta = 6040, // APP_IFACE_PROGRESS_ELAPSED_ETA
        AppInterfaceActOpenFilesNAME = 6010, // APP_IFACE_ACT_OPENFILES_NAME
        AppInterfaceActOpenFilesSKEY = 6011, // APP_IFACE_ACT_OPENFILES_SKEY
        AppInterfaceActOpenFilesSTIP = 6012, // APP_IFACE_ACT_OPENFILES_STIP
        AppInterfaceActOpenFilesDvdName = 6024, // APP_IFACE_ACT_OPENFILES_DVD_NAME
        AppInterfaceActOpenFilesDvdSTIP = 6026, // APP_IFACE_ACT_OPENFILES_DVD_STIP
        AppInterfaceActCloseDiskName = 6013, // APP_IFACE_ACT_CLOSEDISK_NAME
        AppInterfaceActCloseDiskSTIP = 6014, // APP_IFACE_ACT_CLOSEDISK_STIP
        AppInterfaceActSetFolderName = 6015, // APP_IFACE_ACT_SETFOLDER_NAME
        AppInterfaceActSetFolderSTIP = 6016, // APP_IFACE_ACT_SETFOLDER_STIP
        AppInterfaceActSaveAllMkvName = 6017, // APP_IFACE_ACT_SAVEALLMKV_NAME
        AppInterfaceActSaveAllMkvSTIP = 6018, // APP_IFACE_ACT_SAVEALLMKV_STIP
        AppInterfaceActCancelName = 6036, // APP_IFACE_ACT_CANCEL_NAME
        AppInterfaceActCancelSTIP = 6037, // APP_IFACE_ACT_CANCEL_STIP
        AppInterfaceActStreamingName = 6131, // APP_IFACE_ACT_STREAMING_NAME
        AppInterfaceActStreamingSTIP = 6132, // APP_IFACE_ACT_STREAMING_STIP
        AppInterfaceActBackupName = 6133, // APP_IFACE_ACT_BACKUP_NAME
        AppInterfaceActBackupSTIP = 6134, // APP_IFACE_ACT_BACKUP_STIP
        AppInterfaceActQuitName = 6019, // APP_IFACE_ACT_QUIT_NAME
        AppInterfaceActQuitSKey = 6020, // APP_IFACE_ACT_QUIT_SKEY
        AppInterfaceActQuitSTIP = 6021, // APP_IFACE_ACT_QUIT_STIP
        AppInterfaceActAboutName = 6022, // APP_IFACE_ACT_ABOUT_NAME
        AppInterfaceActAboutSTIP = 6023, // APP_IFACE_ACT_ABOUT_STIP
        AppInterfaceActSettingsName = 6042, // APP_IFACE_ACT_SETTINGS_NAME
        AppInterfaceActSettingsSTIP = 6043, // APP_IFACE_ACT_SETTINGS_STIP
        AppInterfaceActHelpPageName = 6045, // APP_IFACE_ACT_HELPPAGE_NAME
        AppInterfaceActHelpPageSTIP = 6046, // APP_IFACE_ACT_HELPPAGE_STIP
        AppInterfaceActRegisterName = 6047, // APP_IFACE_ACT_REGISTER_NAME
        AppInterfaceActRegisterSTIP = 6048, // APP_IFACE_ACT_REGISTER_STIP
        AppInterfaceActPurchaseNAME = 6145, // APP_IFACE_ACT_PURCHASE_NAME
        AppInterfaceActPurchaseSTIP = 6146, // APP_IFACE_ACT_PURCHASE_STIP
        AppInterfaceActClearLogName = 6110, // APP_IFACE_ACT_CLEARLOG_NAME
        AppInterfaceActClearLogSTIP = 6111, // APP_IFACE_ACT_CLEARLOG_STIP
        AppInterfaceActEjectName = 6052, // APP_IFACE_ACT_EJECT_NAME
        AppInterfaceActEjectSTIP = 6053, // APP_IFACE_ACT_EJECT_STIP
        AppInterfaceActRevertNAME = 6105, // APP_IFACE_ACT_REVERT_NAME
        AppInterfaceActRevertSTIP = 6106, // APP_IFACE_ACT_REVERT_STIP
        AppInterfaceActNewInstanceName = 6107, // APP_IFACE_ACT_NEWINSTANCE_NAME
        AppInterfaceActNewInstanceSTIP = 6108, // APP_IFACE_ACT_NEWINSTANCE_STIP
        AppInterfaceActOpenDiscDvd = 6062, // APP_IFACE_ACT_OPENDISC_DVD
        AppInterfaceActOpenDiscHdDvd = 6063, // APP_IFACE_ACT_OPENDISC_HDDVD
        AppInterfaceActOpenDiscBluRay = 6064, // APP_IFACE_ACT_OPENDISC_BRAY
        AppInterfaceActOpenDiscLoading = 6065, // APP_IFACE_ACT_OPENDISC_LOADING
        AppInterfaceActOpenDiscUnknown = 6099, // APP_IFACE_ACT_OPENDISC_UNKNOWN
        AppInterfaceActOpenDiscNoDisc = 6109, // APP_IFACE_ACT_OPENDISC_NODISC
        AppInterfaceActTitleTreeToggle = 6066, // APP_IFACE_ACT_TTREE_TOGGLE
        AppInterfaceActTitleTreeSelectAll = 6067, // APP_IFACE_ACT_TTREE_SELECT_ALL
        AppInterfaceActTitleTreeUnselectAll = 6068, // APP_IFACE_ACT_TTREE_UNSELECT_ALL
        AppInterfaceMenuFile = 6030, // APP_IFACE_MENU_FILE
        AppInterfaceMenuView = 6031, // APP_IFACE_MENU_VIEW
        AppInterfaceMenuHelp = 6032, // APP_IFACE_MENU_HELP
        AppInterfaceMenuToolbar = 6034, // APP_IFACE_MENU_TOOLBAR
        AppInterfaceMenuSettings = 6044, // APP_IFACE_MENU_SETTINGS
        AppInterfaceMenuDRIVES = 6035, // APP_IFACE_MENU_DRIVES
        AppInterfaceCancelCONFIRM = 6041, // APP_IFACE_CANCEL_CONFIRM
        AppInterfaceFatalCOMM = 6050, // APP_IFACE_FATAL_COMM
        AppInterfaceFatalMEM = 6051, // APP_IFACE_FATAL_MEM
        AppInterfaceGuiVERSION = 6054, // APP_IFACE_GUI_VERSION
        AppInterfaceLatestVERSION = 6158, // APP_IFACE_LATEST_VERSION
        AppInterfaceLicenseTYPE = 6055, // APP_IFACE_LICENSE_TYPE
        AppInterfaceEvalSTATE = 6056, // APP_IFACE_EVAL_STATE
        AppInterfaceEvalEXPIRATION = 6057, // APP_IFACE_EVAL_EXPIRATION
        AppInterfaceProgEXPIRATION = 6142, // APP_IFACE_PROG_EXPIRATION
        AppInterfaceWebsiteURL = 6159, // APP_IFACE_WEBSITE_URL
        AppInterfaceVideoFolderNameWin = 6058, // APP_IFACE_VIDEO_FOLDER_NAME_WIN
        AppInterfaceVideoFolderNameMac = 6059, // APP_IFACE_VIDEO_FOLDER_NAME_MAC
        AppInterfaceVideoFolderNameLinux = 6060, // APP_IFACE_VIDEO_FOLDER_NAME_LINUX
        AppInterfaceDefaultFolderName = 6061, // APP_IFACE_DEFAULT_FOLDER_NAME
        AppInterfaceMainFrameInfo = 6069, // APP_IFACE_MAIN_FRAME_INFO
        AppInterfaceMainFrameMakeMkv = 6070, // APP_IFACE_MAIN_FRAME_MAKE_MKV
        AppInterfaceMainFrameProfile = 6180, // APP_IFACE_MAIN_FRAME_PROFILE
        AppInterfaceMainFrameProperties = 6181, // APP_IFACE_MAIN_FRAME_PROPERTIES
        AppInterfaceEmptyFrameInfo = 6075, // APP_IFACE_EMPTY_FRAME_INFO
        AppInterfaceEmptyFrameSource = 6071, // APP_IFACE_EMPTY_FRAME_SOURCE
        AppInterfaceEmptyFrameType = 6072, // APP_IFACE_EMPTY_FRAME_TYPE
        AppInterfaceEmptyFrameLabel = 6073, // APP_IFACE_EMPTY_FRAME_LABEL
        AppInterfaceEmptyFrameProtection = 6074, // APP_IFACE_EMPTY_FRAME_PROTECTION
        AppInterfaceEmptyFrameDvdManual = 6084, // APP_IFACE_EMPTY_FRAME_DVD_MANUAL
        AppInterfaceRegisterTEXT = 6076, // APP_IFACE_REGISTER_TEXT
        AppInterfaceRegisterCodeIncorrect = 6077, // APP_IFACE_REGISTER_CODE_INCORRECT
        AppInterfaceRegisterCodeNotSaved = 6078, // APP_IFACE_REGISTER_CODE_NOT_SAVED
        AppInterfaceRegisterCodeSaved = 6079, // APP_IFACE_REGISTER_CODE_SAVED
        AppInterfaceSettingsIoOptions = 6080, // APP_IFACE_SETTINGS_IO_OPTIONS
        AppInterfaceSettingsIoAuto = 6081, // APP_IFACE_SETTINGS_IO_AUTO
        AppInterfaceSettingsIoReadRetry = 6082, // APP_IFACE_SETTINGS_IO_READ_RETRY
        AppInterfaceSettingsIoReadBuffer = 6083, // APP_IFACE_SETTINGS_IO_READ_BUFFER
        AppInterfaceSettingsIoNoDirectAccess = 6150, // APP_IFACE_SETTINGS_IO_NO_DIRECT_ACCESS
        AppInterfaceSettingsIoDarwinK2Workaround = 6151, // APP_IFACE_SETTINGS_IO_DARWIN_K2_WORKAROUND
        AppInterfaceSettingsIoSingleDrive = 6168, // APP_IFACE_SETTINGS_IO_SINGLE_DRIVE
        AppInterfaceSettingsDvdAuto = 6085, // APP_IFACE_SETTINGS_DVD_AUTO
        AppInterfaceSettingsDvdMinLength = 6086, // APP_IFACE_SETTINGS_DVD_MIN_LENGTH
        AppInterfaceSettingsDvdSpRemove = 6087, // APP_IFACE_SETTINGS_DVD_SP_REMOVE
        AppInterfaceSettingsAacsKeyDir = 6088, // APP_IFACE_SETTINGS_AACS_KEY_DIR
        AppInterfaceSettingsBdpMISC = 6129, // APP_IFACE_SETTINGS_BDP_MISC
        AppInterfaceSettingsBdpDumpAlways = 6130, // APP_IFACE_SETTINGS_BDP_DUMP_ALWAYS
        AppInterfaceSettingsDestTypeNone = 6089, // APP_IFACE_SETTINGS_DEST_TYPE_NONE
        AppInterfaceSettingsDestTypeAuto = 6090, // APP_IFACE_SETTINGS_DEST_TYPE_AUTO
        AppInterfaceSettingsDestTypeSemiAuto = 6091, // APP_IFACE_SETTINGS_DEST_TYPE_SEMIAUTO
        AppInterfaceSettingsDestTypeCustom = 6092, // APP_IFACE_SETTINGS_DEST_TYPE_CUSTOM
        AppInterfaceSettingsDestDir = 6093, // APP_IFACE_SETTINGS_DESTDIR
        AppInterfaceSettingsGeneralMisc = 6094, // APP_IFACE_SETTINGS_GENERAL_MISC
        AppInterfaceSettingsLogDebugMsg = 6095, // APP_IFACE_SETTINGS_LOG_DEBUG_MSG
        AppInterfaceSettingsDataDir = 6167, // APP_IFACE_SETTINGS_DATA_DIR
        AppInterfaceSettingsExpertMode = 6169, // APP_IFACE_SETTINGS_EXPERT_MODE
        AppInterfaceSettingsShowAvSync = 6170, // APP_IFACE_SETTINGS_SHOW_AVSYNC
        AppInterfaceSettingsGeneralOnlineUpdates = 6188, // APP_IFACE_SETTINGS_GENERAL_ONLINE_UPDATES
        AppInterfaceSettingsEnableInternetAccess = 6187, // APP_IFACE_SETTINGS_ENABLE_INTERNET_ACCESS
        AppInterfaceSettingsProxyServer = 6189, // APP_IFACE_SETTINGS_PROXY_SERVER
        AppInterfaceSettingsTabGeneral = 6096, // APP_IFACE_SETTINGS_TAB_GENERAL
        AppInterfaceSettingsMsgFailed = 6097, // APP_IFACE_SETTINGS_MSG_FAILED
        AppInterfaceSettingsMsgRestart = 6098, // APP_IFACE_SETTINGS_MSG_RESTART
        AppInterfaceSettingsTabLanguage = 6152, // APP_IFACE_SETTINGS_TAB_LANGUAGE
        AppInterfaceSettingsLangInterface = 6153, // APP_IFACE_SETTINGS_LANG_INTERFACE
        AppInterfaceSettingsLangPreferred = 6154, // APP_IFACE_SETTINGS_LANG_PREFERRED
        AppInterfaceSettingsLanguageAuto = 6156, // APP_IFACE_SETTINGS_LANGUAGE_AUTO
        AppInterfaceSettingsLanguageNone = 6157, // APP_IFACE_SETTINGS_LANGUAGE_NONE
        AppInterfaceSettingsTabIo = 6164, // APP_IFACE_SETTINGS_TAB_IO
        AppInterfaceSettingsTabStreaming = 6165, // APP_IFACE_SETTINGS_TAB_STREAMING
        AppInterfaceSettingsTabProt = 6166, // APP_IFACE_SETTINGS_TAB_PROT
        AppInterfaceSettingsTabAdvanced = 6172, // APP_IFACE_SETTINGS_TAB_ADVANCED
        AppInterfaceSettingsAdvDefaultProfile = 6173, // APP_IFACE_SETTINGS_ADV_DEFAULT_PROFILE
        AppInterfaceSettingsAdvDefaultSelection = 6174, // APP_IFACE_SETTINGS_ADV_DEFAULT_SELECTION
        AppInterfaceSettingsAdvExternExecPath = 6175, // APP_IFACE_SETTINGS_ADV_EXTERN_EXEC_PATH
        AppInterfaceSettingsProtJavaPath = 6177, // APP_IFACE_SETTINGS_PROT_JAVA_PATH
        AppInterfaceSettingsAdvOutputFileNameTemplate = 6178, // APP_IFACE_SETTINGS_ADV_OUTPUT_FILE_NAME_TEMPLATE
        AppInterfaceSettingsTabIntegration = 6190, // APP_IFACE_SETTINGS_TAB_INTEGRATION
        AppInterfaceSettingsIntText = 6191, // APP_IFACE_SETTINGS_INT_TEXT
        AppInterfaceSettingsIntHdrPath = 6192, // APP_IFACE_SETTINGS_INT_HDR_PATH
        AppInterfaceKeyText = 6179, // APP_IFACE_KEY_TEXT
        AppInterfaceKeyName = 6182, // APP_IFACE_KEY_NAME
        AppInterfaceKeyType = 6183, // APP_IFACE_KEY_TYPE
        AppInterfaceKeyDate = 6184, // APP_IFACE_KEY_DATE
        AppInterfaceBackupDlgTextCaption = 6137, // APP_IFACE_BACKUPDLG_TEXT_CAPTION
        AppInterfaceBackupDlgText = 6138, // APP_IFACE_BACKUPDLG_TEXT
        AppInterfaceBackupDlgFolder = 6139, // APP_IFACE_BACKUPDLG_FOLDER
        AppInterfaceBackupDlgOptions = 6147, // APP_IFACE_BACKUPDLG_OPTIONS
        AppInterfaceBackupDlgDecrypt = 6148, // APP_IFACE_BACKUPDLG_DECRYPT
        AppInterfaceDriveInfoLoading = 6100, // APP_IFACE_DRIVEINFO_LOADING
        AppInterfaceDriveInfoUnmounting = 6112, // APP_IFACE_DRIVEINFO_UNMOUNTING
        AppInterfaceDriveInfoWait = 6101, // APP_IFACE_DRIVEINFO_WAIT
        AppInterfaceDriveInfoNoDisc = 6102, // APP_IFACE_DRIVEINFO_NODISC
        AppInterfaceDriveInfoDataDisc = 6103, // APP_IFACE_DRIVEINFO_DATADISC
        AppInterfaceDriveInfoNone = 6104, // APP_IFACE_DRIVEINFO_NONE
        AppInterfaceFlagsDirectorsComments = 6125, // APP_IFACE_FLAGS_DIRECTORS_COMMENTS
        AppInterfaceFlagsAltDirectorsComments = 6126, // APP_IFACE_FLAGS_ALT_DIRECTORS_COMMENTS
        AppInterfaceFlagsSecondaryAudio = 6127, // APP_IFACE_FLAGS_SECONDARY_AUDIO
        AppInterfaceFlagsForVisuallyImpaired = 6128, // APP_IFACE_FLAGS_FOR_VISUALLY_IMPAIRED
        AppInterfaceFlagsCoreAUDIO = 6143, // APP_IFACE_FLAGS_CORE_AUDIO
        AppInterfaceFlagsForcedSUBTITLES = 6144, // APP_IFACE_FLAGS_FORCED_SUBTITLES
        AppInterfaceFlagsProfileSecondaryStream = 6171, // APP_IFACE_FLAGS_PROFILE_SECONDARY_STREAM
        AppInterfaceItemInfoSource = 6119, // APP_IFACE_ITEMINFO_SOURCE
        AppInterfaceItemInfoTitle = 6120, // APP_IFACE_ITEMINFO_TITLE
        AppInterfaceItemInfoTRACK = 6121, // APP_IFACE_ITEMINFO_TRACK
        AppInterfaceItemInfoATTACHMENT = 6122, // APP_IFACE_ITEMINFO_ATTACHMENT
        AppInterfaceItemInfoCHAPTER = 6123, // APP_IFACE_ITEMINFO_CHAPTER
        AppInterfaceItemInfoCHAPTERS = 6124, // APP_IFACE_ITEMINFO_CHAPTERS
        AppTitleTreeTitle = 6200, // APP_TTREE_TITLE
        AppTitleTreeVideo = 6201, // APP_TTREE_VIDEO
        AppTitleTreeAudio = 6202, // APP_TTREE_AUDIO
        AppTitleTreeSubPicture = 6203, // APP_TTREE_SUBPICTURE
        AppTitleTreeAttachment = 6214, // APP_TTREE_ATTACHMENT
        AppTitleTreeChapters = 6215, // APP_TTREE_CHAPTERS
        AppTitleTreeChapter = 6216, // APP_TTREE_CHAPTER
        AppTitleTreeForcedSubtitles = 6211, // APP_TTREE_FORCED_SUBTITLES
        AppTitleTreeHdrType = 6204, // APP_TTREE_HDR_TYPE
        AppTitleTreeHdrDESC = 6205, // APP_TTREE_HDR_DESC
        DvdTypeDisk = 6206, // DVD_TYPE_DISK
        BluRayTypeDisk = 6209, // BRAY_TYPE_DISK
        HdDvdTypeDisk = 6212, // HDDVD_TYPE_DISK
        MkvTypeFile = 6213, // MKV_TYPE_FILE
        AppTitleTreeChapDESC = 6207, // APP_TTREE_CHAP_DESC
        AppTitleTreeAngleDESC = 6210, // APP_TTREE_ANGLE_DESC
        AppDvdManualTitle = 6220, // APP_DVD_MANUAL_TITLE
        AppDvdManualText = 6225, // APP_DVD_MANUAL_TEXT
        AppDvdTitlesCount = 6221, // APP_DVD_TITLES_COUNT
        AppDvdCountCells = 6222, // APP_DVD_COUNT_CELLS
        AppDvdCountPgc = 6223, // APP_DVD_COUNT_PGC
        AppDvdBrokenTitleEntry = 6224, // APP_DVD_BROKEN_TITLE_ENTRY
        AppSingleDriveTitle = 6226, // APP_SINGLE_DRIVE_TITLE
        AppSingleDriveText = 6227, // APP_SINGLE_DRIVE_TEXT
        AppSingleDriveAll = 6228, // APP_SINGLE_DRIVE_ALL
        AppSingleDriveCaption = 6229, // APP_SINGLE_DRIVE_CAPTION
        AppSiDriveInfo = 6300, // APP_SiDRIVEINFO
        AppSiProfile = 6301, // APP_SiPROFILE
        AppSiManufacturer = 6302, // APP_SiMANUFACTURER
        AppSiProduct = 6303, // APP_SiPRODUCT
        AppSiRevision = 6304, // APP_SiREVISION
        AppSiSerial = 6305, // APP_SiSERIAL
        AppSiFirmware = 6306, // APP_SiFIRMWARE
        AppSiFirDate = 6307, // APP_SiFIRDATE
        AppSiBecFlags = 6308, // APP_SiBECFLAGS
        AppSiHighestAacs = 6309, // APP_SiHIGHEST_AACS
        AppSiDiscInfo = 6320, // APP_SiDISCINFO
        AppSiNoDisc = 6321, // APP_SiNODISC
        AppSiDiscLoad = 6322, // APP_SiDISCLOAD
        AppSiCapacity = 6323, // APP_SiCAPACITY
        AppSiDiscType = 6324, // APP_SiDISCTYPE
        AppSiDiscSize = 6325, // APP_SiDISCSIZE
        AppSiDiscRate = 6326, // APP_SiDISCRATE
        AppSiDiscLayers = 6327, // APP_SiDISCLAYERS
        AppSiDiscCbl = 6329, // APP_SiDISCCBL
        AppSiDiscCbl25 = 6330, // APP_SiDISCCBL25
        AppSiDiscCbl27 = 6331, // APP_SiDISCCBL27
        AppSiDevice = 6332, // APP_SiDEVICE
        // ---------------------------------------------------------------------
        // known to exist but undocumented:
        // 5087
        // 5088
        // 5090
        // 5091
    }
}

impl std::fmt::Display for ItemAttributeCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn parse_item_attribute_id(value: u32) -> Option<ItemAttributeId> {
    ItemAttributeId::from_u32(value)
}

enum_from_primitive! {
    /// item attribute id
    /// 
    /// The values are derived from enum '_AP_ItemAttributeId' in
    /// `makemkv-oss-1.18.3/makemkvgui/inc/lgpl/apdefs.h`.
    #[derive(Clone, Debug, PartialEq, serde::Serialize)]
    pub enum ItemAttributeId {
        Unknown = 0, // ap_iaUnknown
        Type = 1, // ap_iaType
        Name = 2, // ap_iaName
        LangCode = 3, // ap_iaLangCode
        LangName = 4, // ap_iaLangName
        CodecId = 5, // ap_iaCodecId
        CodecShort = 6, // ap_iaCodecShort
        CodecLong = 7, // ap_iaCodecLong
        ChapterCount = 8, // ap_iaChapterCount
        Duration = 9, // ap_iaDuration
        DiskSize = 10, // ap_iaDiskSize
        DiskSizeBytes = 11, // ap_iaDiskSizeBytes
        StreamTypeExtension = 12, // ap_iaStreamTypeExtension
        Bitrate = 13, // ap_iaBitrate
        AudioChannelsCount = 14, // ap_iaAudioChannelsCount
        AngleInfo = 15, // ap_iaAngleInfo
        SourceFileName = 16, // ap_iaSourceFileName
        AudioSampleRate = 17, // ap_iaAudioSampleRate
        AudioSampleSize = 18, // ap_iaAudioSampleSize
        VideoSize = 19, // ap_iaVideoSize
        VideoAspectRatio = 20, // ap_iaVideoAspectRatio
        VideoFrameRate = 21, // ap_iaVideoFrameRate
        StreamFlags = 22, // ap_iaStreamFlags
        DateTime = 23, // ap_iaDateTime
        OriginalTitleId = 24, // ap_iaOriginalTitleId
        SegmentsCount = 25, // ap_iaSegmentsCount
        SegmentsMap = 26, // ap_iaSegmentsMap
        OutputFileName = 27, // ap_iaOutputFileName
        MetadataLanguageCode = 28, // ap_iaMetadataLanguageCode
        MetadataLanguageName = 29, // ap_iaMetadataLanguageName
        TreeInfo = 30, // ap_iaTreeInfo
        PanelTitle = 31, // ap_iaPanelTitle
        VolumeName = 32, // ap_iaVolumeName
        OrderWeight = 33, // ap_iaOrderWeight
        OutputFormat = 34, // ap_iaOutputFormat
        OutputFormatDescription = 35, // ap_iaOutputFormatDescription
        SeamlessInfo = 36, // ap_iaSeamlessInfo
        PanelText = 37, // ap_iaPanelText
        MkvFlags = 38, // ap_iaMkvFlags
        MkvFlagsText = 39, // ap_iaMkvFlagsText
        AudioChannelLayoutName = 40, // ap_iaAudioChannelLayoutName
        OutputCodecShort = 41, // ap_iaOutputCodecShort
        OutputConversionType = 42, // ap_iaOutputConversionType
        OutputAudioSampleRate = 43, // ap_iaOutputAudioSampleRate
        OutputAudioSampleSize = 44, // ap_iaOutputAudioSampleSize
        OutputAudioChannelsCount = 45, // ap_iaOutputAudioChannelsCount
        OutputAudioChannelLayoutName = 46, // ap_iaOutputAudioChannelLayoutName
        OutputAudioChannelLayout = 47, // ap_iaOutputAudioChannelLayout
        OutputAudioMixDescription = 48, // ap_iaOutputAudioMixDescription
        Comment = 49, // ap_iaComment
        OffsetSequenceId = 50, // ap_iaOffsetSequenceId
    }
}

impl std::fmt::Display for ItemAttributeId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn parse_item_attribute_code(value: u32) -> Option<ItemAttributeCode> {
    ItemAttributeCode::from_u32(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attr_code_to_string() {
        let data = ItemAttributeCode::AppDumpDonePartial;
        // ----------------------------------------------------------------
        let computed = data.to_string();
        let expected = "AppDumpDonePartial".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_attr_id_to_string() {
        let data = ItemAttributeId::Bitrate;
        // ----------------------------------------------------------------
        let computed = data.to_string();
        let expected = "Bitrate".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

}
