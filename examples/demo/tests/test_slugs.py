from shop.slugs import slugify


def test_slugify():
    assert slugify("Green Tea, Loose!") == "green-tea-loose"


def test_slugify_accents():
    assert slugify("Café Crème") == "cafe-creme"
