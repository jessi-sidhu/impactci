from .cart import Cart


def percent_off(cart: Cart, pct: int) -> int:
    return cart.total() * (100 - pct) // 100
