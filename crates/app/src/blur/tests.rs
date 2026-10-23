use drakeflake_config::BlurMode;

use super::BlurRequest;

#[test]
fn auto_probes_quietly_and_on_warns() {
    assert_eq!(BlurRequest::from(BlurMode::Auto), BlurRequest::Probe);
    assert_eq!(BlurRequest::from(BlurMode::On), BlurRequest::Require);
    assert_eq!(BlurRequest::from(BlurMode::Off), BlurRequest::Off);
}

#[test]
fn the_request_maps_to_the_two_flags_of_the_cpp_side() {
    let flags = |request: BlurRequest| (request.enabled(), request.silent());

    assert_eq!(flags(BlurRequest::Probe), (true, true));
    assert_eq!(flags(BlurRequest::Require), (true, false));
    assert_eq!(flags(BlurRequest::Off), (false, false));
}
