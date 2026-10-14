from shop.money import fmt, to_cents


def test_to_cents():
    assert to_cents("1.005") == 101


def test_fmt():
    assert fmt(1234) == "$12.34"
