/*
    show structure of a physical medium or disc image

    This code wraps 'makemkvcon info' into a more convenient interface.
*/

use std::path::Path;

pub use makemkv::ScanResult;

pub fn scan_disc(makemkvcon_bin: &Path, source: &str) -> Option<ScanResult> {
    // Setting "min_length" is a crude way to filter the content. It forces
    // the user to decide whether to use a high value to reliably exclude
    // advertisements (and accidentally skip short feature-related content)
    // or use a low value to ensure all feature-related content is captured
    // at the cost of letting more unrelated advertisements slip through.
    //
    // decision:
    // - force minimum length to 0 seconds
    // - provide ability to filter in a different, more suitable way
    let min_length = 0;

    makemkv::info(makemkvcon_bin, source, min_length)
}
