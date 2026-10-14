from shop.cart import Cart


def test_total():
    c = Cart()
    c.add("tea", "3.50", 2)
    assert c.total() == 700
