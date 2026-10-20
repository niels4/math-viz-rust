"""Image comparisons. A pixel counts as changed when its largest channel difference exceeds the
threshold, which every result states: 0 for a claim that two images are the same."""

from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFont

Box = tuple[int, int, int, int]  # x, y, width, height


@dataclass
class Difference:
    """Two images of the same size compared over their RGB channels."""

    width: int
    height: int
    threshold: int  # a pixel changed when a channel differs by more than this, of 255
    changed: int  # pixels changed
    max: int  # largest channel difference
    mean: float  # mean channel difference over the whole image, of 255
    bounds: Box | None  # the smallest box holding every changed pixel
    inside: int | None = None  # with a box: pixels changed inside it
    outside: int | None = None  # and outside it

    @property
    def same(self) -> bool:
        return self.changed == 0

    def summary(self) -> str:
        share = self.changed / (self.width * self.height) * 100
        text = (
            f"{self.width}x{self.height}, threshold {self.threshold}/255: {self.changed} pixels "
            f"changed ({share:.3f}%), max {self.max}/255, mean {self.mean:.3f}"
        )
        if self.bounds is not None:
            x, y, w, h = self.bounds
            text += f", all within ({x},{y}) {w}x{h}"
        if self.inside is not None:
            text += f"; inside the box {self.inside}, outside {self.outside}"
        return text


def load_rgb(path: Path) -> Image.Image:
    with Image.open(path) as image:
        return image.convert("RGB")


def changed_mask(a: Image.Image, b: Image.Image, threshold: int) -> Image.Image:
    """A one-channel image, 255 where a pixel changed by more than `threshold`."""
    diff = ImageChops.difference(a, b)
    red, green, blue = diff.split()
    largest = ImageChops.lighter(ImageChops.lighter(red, green), blue)
    table = [255 if v > threshold else 0 for v in range(256)]
    return largest.point(table)  # pyright: ignore[reportUnknownMemberType]


def count_set(mask: Image.Image) -> int:
    return mask.histogram()[255]


def difference(
    a: Image.Image,
    b: Image.Image,
    threshold: int = 0,
    box: Box | None = None,
    out: Path | None = None,
) -> Difference:
    """Compare `a` with `b`. With `box`, count the changed pixels inside and outside it. With
    `out`, write the changed pixels in white over a dimmed `b`."""
    if a.size != b.size:
        raise ValueError(f"sizes differ: {a.size[0]}x{a.size[1]} and {b.size[0]}x{b.size[1]}")
    diff = ImageChops.difference(a, b)
    total = 0
    largest = 0
    for channel in diff.split():
        histogram = channel.histogram()
        total += sum(value * count for value, count in enumerate(histogram))
        largest = max([largest, *(v for v, count in enumerate(histogram) if count > 0)])
    width, height = a.size
    mask = changed_mask(a, b, threshold)
    changed = count_set(mask)
    found = mask.getbbox()
    bounds = (
        None if found is None else (found[0], found[1], found[2] - found[0], found[3] - found[1])
    )
    result = Difference(
        width, height, threshold, changed, largest, total / (width * height * 3), bounds
    )
    if box is not None:
        x, y, w, h = box
        result.inside = count_set(mask.crop((x, y, x + w, y + h)))
        result.outside = changed - result.inside
    if out is not None:
        shown = Image.blend(Image.new("RGB", b.size), b, 0.35)
        shown.paste((255, 255, 255), mask=mask)
        if box is not None:
            x, y, w, h = box
            ImageDraw.Draw(shown).rectangle((x - 1, y - 1, x + w, y + h), outline=(255, 64, 64))
        shown.save(out)
    return result


def region(image: Image.Image, box: Box) -> Image.Image:
    """The part of `image` inside `box`, in the image's pixels."""
    x, y, w, h = box
    return image.crop((x, y, x + w, y + h))


def scaled_box(box: Box, density: float) -> Box:
    """A box in logical pixels, in device pixels at `density`."""
    x, y, w, h = box
    return (round(x * density), round(y * density), round(w * density), round(h * density))


def contact_sheet(
    rows: list[tuple[str, list[tuple[str, Path | None]]]],
    out: Path,
    cell_width: int = 480,
) -> None:
    """A grid of images: one row per label, each cell a captioned image (or a gap where it is
    missing), all scaled to `cell_width` across."""
    font = ImageFont.load_default(size=18)
    caption = 28
    sizes = [load_rgb(p).size for _, cells in rows for _, p in cells if p is not None]
    if not sizes:
        return
    aspect = sizes[0][1] / sizes[0][0]
    cell_height = round(cell_width * aspect)
    label_width = 220
    columns = max(len(cells) for _, cells in rows)
    sheet = Image.new(
        "RGB",
        (label_width + columns * (cell_width + 8), len(rows) * (cell_height + caption + 8)),
        (24, 24, 24),
    )
    draw = ImageDraw.Draw(sheet)
    for r, (label, cells) in enumerate(rows):
        top = r * (cell_height + caption + 8)
        draw.text((8, top + caption + cell_height // 2), label, fill=(230, 230, 230), font=font)
        for c, (title, path) in enumerate(cells):
            left = label_width + c * (cell_width + 8)
            draw.text((left, top + 4), title, fill=(200, 200, 200), font=font)
            if path is None:
                continue
            thumb = load_rgb(path).resize(  # pyright: ignore[reportUnknownMemberType]
                (cell_width, cell_height), Image.Resampling.LANCZOS
            )
            sheet.paste(thumb, (left, top + caption))
    sheet.save(out)
