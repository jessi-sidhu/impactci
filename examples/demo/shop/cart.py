from .money import to_cents


class Cart:
    def __init__(self):
        self.items: list[tuple[str, int, int]] = []

    def add(self, name: str, price: str, qty: int = 1) -> None:
        self.items.append((name, to_cents(price), qty))

    def total(self) -> int:
        return sum(price * qty for _, price, qty in self.items)
