from shop.cart import Cart
from shop.discounts import percent_off


def test_percent_off():
    c = Cart()
    c.add("tea", "10.00")
    assert percent_off(c, 25) == 750
