/*
    This value controls the '--noscan' command line argument.

    This setting changes the information included in 'DRV' records.

    Best Practice: Always use `DriveOnly` unless you need to know the
    drive's index or the disc's content type (DVD, HD-DVD or Blu-Ray).

    -----------------------------------------------------------------------

    Unfortunately makemkvcon has no dedicated mode to identify drives and
    read the inserted disc. By default, makemkvcon ALWAYS scans all drives
    and attempts to read from all discs, even if it's not necessary and
    would negatively impact concurrent I/O operations on other drives.

    Depending on how many drives there are and if the drives are busy this
    scanning may take anything between 10 seconds to 2 minutes to complete.
*/

/// Configure makemkvcon's drive scan mode.
/// 
/// This setting changes the information included in 'DRV' records.
///
/// **Best Practice:** Always use `ScanMode::DriveOnly` unless you need to
/// know the disc's content type (DVD, HD-DVD or Blu-Ray) or the drive's
/// current state (open, empty, loading, loaded).
/// 
/// **Please note:** The disc's name can be obtained via the DRV record but
/// it's faster to ask the operating system (e.g. use `blkid` on Linux).
#[derive(Default)]
pub enum ScanMode {
    /// Identify the drive; do not scan inserted disc.
    /// 
    /// This operation is relatively quick (less than 10 seconds) and does
    /// not generate I/O requests on these drives. Use this mode if you
    /// don't need the information at all or the only thing you care about
    /// is the drive itself.
    DriveOnly,

    /// Identify the drive and scan inserted disc (default).
    /// 
    /// This operation is slow because it accesses all discs in all drives.
    /// Depending on how many drives there are and if the drives are busy
    /// this operation can take anything between 10 seconds to 2 minutes to
    /// complete.
    /// 
    /// Use this mode if you need to know whether a disc is inserted and
    /// loaded and/or the disc's content type (DVD, HD-DVD or Blu-Ray).
    #[default]
    DriveAndDisc,
}
