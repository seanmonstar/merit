#[test]
fn get_index() {
    let _ = pretty_env_logger::try_init();

    let res = warp::test::request()
        .reply(&crate::index());

    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["content-type"], "text/html; charset=utf-8");
}

#[test]
fn get_badge() {
    let _ = pretty_env_logger::try_init();

    let res = warp::test::request()
        .path("/warp")
        .reply(&crate::badge());

    assert_eq!(res.status(), 302);
    assert_eq!(res.headers()["expires"], "Sun, 01 Jan 1990 00:00:00 GMT");
    assert_eq!(res.headers()["pragma"], "no-cache");
    assert_eq!(
        res.headers()["cache-control"],
        "no-cache, no-store, max-age=0, must-revalidate"
    );
    assert_eq!(
        res.headers()["location"],
        "https://img.shields.io/badge/crates.io-v0.1.9-orange.svg"
    );
}

#[test]
fn get_badge_404() {
    let _ = pretty_env_logger::try_init();

    let res = warp::test::request()
        .path("/this-crate-should-never-exist-amirite")
        .reply(&crate::badge());

    assert_eq!(res.status(), 404);
}

#[test]
fn badge_style() {
    let _ = pretty_env_logger::try_init();

    let style = warp::test::request()
        .path("/warp?style=flat-square")
        .filter(&crate::style())
        .expect("filter flat-square");

    assert_eq!(style, Some(crate::Style::FlatSquare));

    let style = warp::test::request()
        .path("/warp?style=flat-square&warp=speed")
        .filter(&crate::style())
        .expect("filter with unknown query param");

    assert_eq!(style, Some(crate::Style::FlatSquare));

    assert!(
        !warp::test::request()
        .path("/warp?style=new-phone-who-dis")
        .matches(&crate::style()),
        "unknown style query param should reject",
    );

    let style = warp::test::request()
        .path("/warp")
        .filter(&crate::style())
        .expect("filter no query");

    assert_eq!(style, None);
}
