use spoofing::level2::{BookStats, Snap};
use spoofing::shared::Side;

#[test]
fn test_get_average_quantity() {
    let s1 = Snap { level: 0, quantity: 6, timestamp: 1, side: Side::Buy };
    let s2 = Snap { level: 0, quantity: 2, timestamp: 20, side: Side::Buy };
    let s3 = Snap { level: 0, quantity: 4, timestamp: 30, side: Side::Buy };
    let mut foo = BookStats::new(1);
    foo.push(s1).unwrap();
    foo.push(s2).unwrap();
    foo.push(s3).unwrap();
    let left = foo.get_average_quantity(Side::Buy, 0, 25).unwrap();
    assert_eq!(left, 3);
}

#[test]
fn test_push_snap_with_exceed_level() {
    let s1 = Snap { level: 1, quantity: 6, timestamp: 1, side: Side::Buy };
    let mut foo = BookStats::new(1);
    let left = foo.push(s1);
    assert!(left.is_err());
}

#[test]
fn push_earlier_snap_after_older_one() {
    let s1 = Snap { level: 0, quantity: 6, timestamp: 2, side: Side::Buy };
    let s2 = Snap { level: 0, quantity: 6, timestamp: 1, side: Side::Buy };
    let mut foo = BookStats::new(1);
    foo.push(s1).unwrap();
    let left = foo.push(s2);
    assert!(left.is_err());
}
