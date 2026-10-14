from shop.slugs import slugify


def test_slugify():
    assert slugify("Green Tea, Loose!") == "green-tea-loose"
