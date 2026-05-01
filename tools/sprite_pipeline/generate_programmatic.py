#!/usr/bin/env python3
"""
generate_programmatic.py — Generate all Lithos sprites programmatically using Pillow.

Creates crisp vector-art style sprites: flat colors, bold dark outlines,
geometric shapes, transparent backgrounds. Top-down view for all entities.

Usage:
    python generate_programmatic.py --all
    python generate_programmatic.py --category entities
    python generate_programmatic.py --key rover
"""

import argparse
import math
from pathlib import Path

from PIL import Image, ImageDraw

# Output base directory
OUT_DIR = Path(__file__).parent / "../../client/public/sprites"
TRIM_PADDING = 2

# Color palette
PALETTE = {
    "outline": (17, 17, 17, 255),
    "player_suit": (220, 220, 230, 255),
    "player_visor": (50, 100, 200, 255),
    "rover_body": (200, 60, 60, 255),
    "rover_accent": (150, 40, 40, 255),
    "drone_body": (230, 200, 60, 255),
    "drone_accent": (180, 150, 40, 255),
    "assault_body": (230, 130, 50, 255),
    "assault_accent": (180, 90, 30, 255),
    "sniper_body": (60, 200, 220, 255),
    "sniper_accent": (40, 150, 170, 255),
    "heavy_body": (160, 40, 40, 255),
    "heavy_accent": (120, 30, 30, 255),
    "siege_body": (120, 130, 140, 255),
    "siege_accent": (90, 100, 110, 255),
    "warden_body": (80, 30, 30, 255),
    "warden_accent": (200, 50, 50, 255),
    "warden_core": (255, 80, 80, 255),
    "trader_body": (60, 180, 100, 255),
    "trader_accent": (40, 140, 70, 255),
    "item_glow": (200, 160, 255, 255),
    "item_accent": (160, 120, 220, 255),
    "iron": (140, 145, 150, 255),
    "iron_light": (180, 185, 190, 255),
    "copper": (180, 110, 60, 255),
    "copper_light": (220, 150, 90, 255),
    "silica": (200, 210, 220, 255),
    "silica_light": (230, 240, 250, 255),
    "uranium": (40, 45, 40, 255),
    "uranium_glow": (80, 255, 80, 255),
    "plutonium": (45, 40, 50, 255),
    "plutonium_glow": (150, 80, 255, 255),
    "biomass": (60, 160, 60, 255),
    "biomass_light": (100, 220, 100, 255),
    "wall": (100, 105, 110, 255),
    "wall_light": (140, 145, 150, 255),
    "door": (90, 95, 100, 255),
    "door_light": (130, 200, 130, 255),
    "generator": (100, 110, 100, 255),
    "gen_core": (80, 255, 120, 255),
    "workbench": (120, 100, 80, 255),
    "wb_screen": (80, 200, 255, 255),
    "projectile": (255, 160, 40, 255),
    "laser": (60, 220, 255, 255),
    "mining_laser": (200, 255, 60, 255),
    "explosion": (255, 120, 40, 255),
    "explosion_light": (255, 200, 60, 255),
    "smoke": (120, 120, 120, 255),
    "fire": (255, 100, 30, 255),
    "fire_light": (255, 180, 50, 255),
    "spark": (255, 255, 200, 255),
    "warp": (60, 150, 255, 255),
    "white": (255, 255, 255, 255),
}


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def new_image(size: int, square: bool = True):
    """Create a new transparent RGBA image."""
    if square:
        return Image.new("RGBA", (size, size), (0, 0, 0, 0))
    return Image.new("RGBA", size, (0, 0, 0, 0))


def circle(draw: ImageDraw.Draw, cx: float, cy: float, r: float, fill, outline=None, width=2):
    """Draw a circle."""
    draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=fill, outline=outline or PALETTE["outline"], width=width)


def rect(draw: ImageDraw.Draw, x, y, w, h, fill, outline=None, width=2):
    """Draw a rectangle."""
    draw.rectangle([x, y, x + w, y + h], fill=fill, outline=outline or PALETTE["outline"], width=width)


def polygon(draw: ImageDraw.Draw, points, fill, outline=None, width=2):
    """Draw a polygon."""
    draw.polygon(points, fill=fill, outline=outline or PALETTE["outline"], width=width)


def line(draw: ImageDraw.Draw, x1, y1, x2, y2, fill, width=2):
    """Draw a line."""
    draw.line([(x1, y1), (x2, y2)], fill=fill, width=width)


def rounded_rect(draw: ImageDraw.Draw, x, y, w, h, radius, fill, outline=None, width=2):
    """Draw a rounded rectangle."""
    draw.rounded_rectangle([x, y, x + w, y + h], radius=radius, fill=fill, outline=outline or PALETTE["outline"], width=width)


def trim_transparent_bounds(img: Image.Image, padding: int = TRIM_PADDING) -> Image.Image:
    alpha = img.getchannel("A")
    bbox = alpha.getbbox()
    if bbox is None:
        return img
    left, top, right, bottom = bbox
    left = max(0, left - padding)
    top = max(0, top - padding)
    right = min(img.width, right + padding)
    bottom = min(img.height, bottom + padding)
    return img.crop((left, top, right, bottom))


# ---------------------------------------------------------------------------
# Entity Sprites
# ---------------------------------------------------------------------------

def draw_player(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Body
    circle(d, cx, cy, size * 0.35, PALETTE["player_suit"])
    # Backpack / life support
    rounded_rect(d, cx - size * 0.15, cy - size * 0.4, size * 0.3, size * 0.2, 3, PALETTE["player_suit"])
    # Visor
    circle(d, cx, cy - size * 0.05, size * 0.18, PALETTE["player_visor"])
    # Highlight
    circle(d, cx - 3, cy - size * 0.08, 3, PALETTE["white"])
    return img


def draw_rover(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Wheels
    circle(d, cx - size * 0.22, cy + size * 0.12, size * 0.12, PALETTE["rover_accent"])
    circle(d, cx + size * 0.22, cy + size * 0.12, size * 0.12, PALETTE["rover_accent"])
    # Body
    rounded_rect(d, cx - size * 0.28, cy - size * 0.15, size * 0.56, size * 0.35, 4, PALETTE["rover_body"])
    # Antenna
    line(d, cx, cy - size * 0.15, cx, cy - size * 0.35, PALETTE["outline"], width=2)
    circle(d, cx, cy - size * 0.37, 2, PALETTE["white"])
    # Eye / sensor
    circle(d, cx, cy - size * 0.02, size * 0.08, PALETTE["white"])
    circle(d, cx, cy - size * 0.02, size * 0.04, PALETTE["outline"])
    return img


def draw_drone(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Rotor arms
    line(d, cx - size * 0.3, cy, cx + size * 0.3, cy, PALETTE["outline"], width=3)
    line(d, cx, cy - size * 0.3, cx, cy + size * 0.3, PALETTE["outline"], width=3)
    # Rotor hubs
    circle(d, cx - size * 0.3, cy, 4, PALETTE["drone_accent"])
    circle(d, cx + size * 0.3, cy, 4, PALETTE["drone_accent"])
    circle(d, cx, cy - size * 0.3, 4, PALETTE["drone_accent"])
    circle(d, cx, cy + size * 0.3, 4, PALETTE["drone_accent"])
    # Body
    circle(d, cx, cy, size * 0.22, PALETTE["drone_body"])
    # Eye
    circle(d, cx, cy, size * 0.08, PALETTE["white"])
    circle(d, cx, cy, size * 0.04, PALETTE["outline"])
    return img


def draw_assault_walker(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Legs
    rounded_rect(d, cx - size * 0.2, cy + size * 0.1, size * 0.08, size * 0.25, 2, PALETTE["assault_accent"])
    rounded_rect(d, cx + size * 0.12, cy + size * 0.1, size * 0.08, size * 0.25, 2, PALETTE["assault_accent"])
    # Body (stocky)
    rounded_rect(d, cx - size * 0.22, cy - size * 0.2, size * 0.44, size * 0.35, 5, PALETTE["assault_body"])
    # Shotgun barrel
    rect(d, cx + size * 0.12, cy - size * 0.05, size * 0.25, size * 0.08, PALETTE["assault_accent"])
    # Head
    circle(d, cx - size * 0.05, cy - size * 0.22, size * 0.12, PALETTE["assault_accent"])
    # Eye
    circle(d, cx - size * 0.08, cy - size * 0.24, size * 0.04, PALETTE["white"])
    return img


def draw_sniper_walker(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Legs (slim)
    rect(d, cx - size * 0.12, cy + size * 0.05, size * 0.06, size * 0.3, PALETTE["sniper_accent"])
    rect(d, cx + size * 0.06, cy + size * 0.05, size * 0.06, size * 0.3, PALETTE["sniper_accent"])
    # Body (tall slim)
    rounded_rect(d, cx - size * 0.12, cy - size * 0.25, size * 0.24, size * 0.35, 4, PALETTE["sniper_body"])
    # Railgun barrel (long)
    rect(d, cx + size * 0.05, cy - size * 0.15, size * 0.35, size * 0.06, PALETTE["sniper_accent"])
    # Scope bump
    circle(d, cx + size * 0.2, cy - size * 0.18, 3, PALETTE["white"])
    # Head
    circle(d, cx, cy - size * 0.28, size * 0.1, PALETTE["sniper_accent"])
    # Eye
    circle(d, cx + size * 0.04, cy - size * 0.3, size * 0.03, PALETTE["white"])
    return img


def draw_heavy_flamethrower(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Legs (wide stance)
    rounded_rect(d, cx - size * 0.25, cy + size * 0.1, size * 0.1, size * 0.25, 2, PALETTE["heavy_accent"])
    rounded_rect(d, cx + size * 0.15, cy + size * 0.1, size * 0.1, size * 0.25, 2, PALETTE["heavy_accent"])
    # Body (bulky)
    rounded_rect(d, cx - size * 0.25, cy - size * 0.18, size * 0.5, size * 0.32, 6, PALETTE["heavy_body"])
    # Fuel tanks
    rounded_rect(d, cx - size * 0.08, cy - size * 0.28, size * 0.16, size * 0.12, 3, PALETTE["heavy_accent"])
    # Flamethrower nozzle
    rect(d, cx + size * 0.15, cy + size * 0.05, size * 0.2, size * 0.1, PALETTE["heavy_accent"])
    # Head / helmet
    circle(d, cx - size * 0.05, cy - size * 0.18, size * 0.12, PALETTE["heavy_accent"])
    # Visor
    rect(d, cx - size * 0.12, cy - size * 0.22, size * 0.18, size * 0.05, PALETTE["white"])
    return img


def draw_siege_unit(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Treads
    rounded_rect(d, cx - size * 0.35, cy + size * 0.05, size * 0.7, size * 0.22, 3, PALETTE["siege_accent"])
    # Tread details
    for i in range(-2, 3):
        line(d, cx + i * size * 0.1, cy + size * 0.07, cx + i * size * 0.1, cy + size * 0.24, PALETTE["outline"], width=1)
    # Body
    rounded_rect(d, cx - size * 0.25, cy - size * 0.2, size * 0.5, size * 0.3, 5, PALETTE["siege_body"])
    # Cannon barrel
    rect(d, cx + size * 0.15, cy - size * 0.12, size * 0.3, size * 0.1, PALETTE["siege_accent"])
    # Turret
    circle(d, cx, cy - size * 0.15, size * 0.12, PALETTE["siege_accent"])
    # Eye / sensor
    circle(d, cx, cy - size * 0.18, size * 0.04, PALETTE["white"])
    return img


def draw_core_warden(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Base ring
    circle(d, cx, cy, size * 0.38, PALETTE["warden_body"])
    # Inner ring
    circle(d, cx, cy, size * 0.28, PALETTE["warden_accent"])
    # Core glow
    circle(d, cx, cy, size * 0.15, PALETTE["warden_core"])
    # Weapon arms
    for angle in [0, 72, 144, 216, 288]:
        rad = math.radians(angle)
        x1 = cx + math.cos(rad) * size * 0.15
        y1 = cy + math.sin(rad) * size * 0.15
        x2 = cx + math.cos(rad) * size * 0.38
        y2 = cy + math.sin(rad) * size * 0.38
        line(d, x1, y1, x2, y2, PALETTE["warden_accent"], width=4)
        circle(d, x2, y2, 4, PALETTE["white"])
    # Center eye
    circle(d, cx, cy, size * 0.06, PALETTE["white"])
    circle(d, cx, cy, size * 0.03, PALETTE["outline"])
    return img


def draw_trader(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Body
    rounded_rect(d, cx - size * 0.18, cy - size * 0.05, size * 0.36, size * 0.3, 4, PALETTE["trader_body"])
    # Head
    circle(d, cx, cy - size * 0.18, size * 0.14, PALETTE["trader_accent"])
    # Backpack
    rounded_rect(d, cx - size * 0.08, cy - size * 0.25, size * 0.16, size * 0.15, 3, PALETTE["trader_accent"])
    # Eye
    circle(d, cx, cy - size * 0.2, size * 0.04, PALETTE["white"])
    # Arms
    line(d, cx - size * 0.18, cy, cx - size * 0.28, cy + size * 0.1, PALETTE["trader_accent"], width=3)
    line(d, cx + size * 0.18, cy, cx + size * 0.28, cy + size * 0.1, PALETTE["trader_accent"], width=3)
    return img


def draw_item_drop(size=32):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Crate body
    rounded_rect(d, cx - size * 0.35, cy - size * 0.3, size * 0.7, size * 0.6, 2, PALETTE["item_glow"])
    # Cross pattern
    line(d, cx - size * 0.2, cy - size * 0.2, cx + size * 0.2, cy + size * 0.2, PALETTE["item_accent"], width=2)
    line(d, cx + size * 0.2, cy - size * 0.2, cx - size * 0.2, cy + size * 0.2, PALETTE["item_accent"], width=2)
    # Glow dots
    circle(d, cx - size * 0.25, cy - size * 0.25, 1.5, PALETTE["white"])
    circle(d, cx + size * 0.25, cy + size * 0.25, 1.5, PALETTE["white"])
    return img


# ---------------------------------------------------------------------------
# Resource Sprites
# ---------------------------------------------------------------------------

def draw_node_iron(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Main rock
    polygon(d, [
        (cx - size * 0.25, cy - size * 0.2),
        (cx + size * 0.1, cy - size * 0.3),
        (cx + size * 0.3, cy),
        (cx + size * 0.1, cy + size * 0.25),
        (cx - size * 0.2, cy + size * 0.15),
    ], PALETTE["iron"])
    # Shards / highlights
    polygon(d, [
        (cx - size * 0.1, cy - size * 0.15),
        (cx + size * 0.05, cy - size * 0.2),
        (cx + size * 0.1, cy - size * 0.05),
    ], PALETTE["iron_light"])
    return img


def draw_node_copper(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Main rock
    polygon(d, [
        (cx - size * 0.2, cy - size * 0.25),
        (cx + size * 0.25, cy - size * 0.15),
        (cx + size * 0.2, cy + size * 0.2),
        (cx - size * 0.15, cy + size * 0.25),
    ], PALETTE["copper"])
    # Oxidized patches
    circle(d, cx - size * 0.08, cy - size * 0.1, size * 0.1, PALETTE["copper_light"])
    circle(d, cx + size * 0.1, cy + size * 0.05, size * 0.08, PALETTE["copper_light"])
    return img


def draw_node_silica(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Crystal cluster
    polygon(d, [
        (cx, cy - size * 0.3),
        (cx + size * 0.15, cy - size * 0.05),
        (cx + size * 0.05, cy + size * 0.2),
        (cx - size * 0.05, cy + size * 0.2),
        (cx - size * 0.15, cy - size * 0.05),
    ], PALETTE["silica"])
    # Inner crystal
    polygon(d, [
        (cx, cy - size * 0.15),
        (cx + size * 0.07, cy),
        (cx, cy + size * 0.1),
        (cx - size * 0.07, cy),
    ], PALETTE["silica_light"])
    return img


def draw_node_uranium(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Dark rock
    polygon(d, [
        (cx - size * 0.25, cy - size * 0.15),
        (cx + size * 0.2, cy - size * 0.2),
        (cx + size * 0.3, cy + size * 0.1),
        (cx, cy + size * 0.25),
        (cx - size * 0.2, cy + size * 0.1),
    ], PALETTE["uranium"])
    # Glow veins
    line(d, cx - size * 0.1, cy - size * 0.05, cx + size * 0.05, cy + size * 0.1, PALETTE["uranium_glow"], width=3)
    line(d, cx + size * 0.05, cy - size * 0.1, cx - size * 0.05, cy + size * 0.15, PALETTE["uranium_glow"], width=2)
    circle(d, cx, cy, 3, PALETTE["uranium_glow"])
    return img


def draw_node_plutonium(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Dark rock
    polygon(d, [
        (cx - size * 0.2, cy - size * 0.2),
        (cx + size * 0.25, cy - size * 0.1),
        (cx + size * 0.15, cy + size * 0.25),
        (cx - size * 0.25, cy + size * 0.15),
    ], PALETTE["plutonium"])
    # Purple glow veins
    line(d, cx - size * 0.05, cy - size * 0.15, cx + size * 0.1, cy, PALETTE["plutonium_glow"], width=3)
    line(d, cx - size * 0.15, cy + size * 0.05, cx + size * 0.05, cy + size * 0.15, PALETTE["plutonium_glow"], width=2)
    circle(d, cx + size * 0.05, cy - size * 0.05, 3, PALETTE["plutonium_glow"])
    return img


def draw_node_biomass(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Organic blobs
    circle(d, cx - size * 0.1, cy - size * 0.1, size * 0.18, PALETTE["biomass"])
    circle(d, cx + size * 0.12, cy + size * 0.05, size * 0.15, PALETTE["biomass"])
    circle(d, cx, cy + size * 0.15, size * 0.12, PALETTE["biomass_light"])
    # Spots
    circle(d, cx - size * 0.08, cy - size * 0.08, 3, PALETTE["biomass_light"])
    circle(d, cx + size * 0.15, cy + size * 0.05, 2.5, PALETTE["biomass_light"])
    circle(d, cx + size * 0.05, cy + size * 0.2, 2, PALETTE["white"])
    return img


# ---------------------------------------------------------------------------
# Structure Sprites
# ---------------------------------------------------------------------------

def draw_wall_segment(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    margin = 4
    rect(d, margin, margin, size - margin * 2, size - margin * 2, PALETTE["wall"])
    # Rivets
    rivet_r = 2
    for x in [margin + 6, size // 2, size - margin - 6]:
        for y in [margin + 6, size - margin - 6]:
            circle(d, x, y, rivet_r, PALETTE["wall_light"])
    # Panel seam
    line(d, size // 2, margin, size // 2, size - margin, PALETTE["wall_light"], width=1)
    return img


def draw_door(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    margin = 4
    rect(d, margin, margin, size - margin * 2, size - margin * 2, PALETTE["door"])
    # Door seam (sliding)
    line(d, size // 2 - 2, margin + 4, size // 2 - 2, size - margin - 4, PALETTE["outline"], width=2)
    # Frame
    rect(d, margin, margin, size - margin * 2, size - margin * 2, None, width=3)
    # Indicator light
    circle(d, size - margin - 8, margin + 8, 3, PALETTE["door_light"])
    return img


def draw_generator(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Main box
    rounded_rect(d, cx - size * 0.3, cy - size * 0.25, size * 0.6, size * 0.5, 4, PALETTE["generator"])
    # Core glow
    circle(d, cx, cy, size * 0.15, PALETTE["gen_core"])
    # Vents
    for i in range(-1, 2):
        line(d, cx - size * 0.2, cy + i * 6, cx + size * 0.2, cy + i * 6, PALETTE["outline"], width=2)
    # Cables
    line(d, cx + size * 0.25, cy + size * 0.15, cx + size * 0.35, cy + size * 0.25, PALETTE["outline"], width=2)
    line(d, cx - size * 0.25, cy + size * 0.15, cx - size * 0.35, cy + size * 0.25, PALETTE["outline"], width=2)
    return img


def draw_workbench(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Table top
    rect(d, cx - size * 0.35, cy - size * 0.05, size * 0.7, size * 0.3, PALETTE["workbench"])
    # Legs
    rect(d, cx - size * 0.3, cy + size * 0.25, size * 0.06, size * 0.15, PALETTE["workbench"])
    rect(d, cx + size * 0.24, cy + size * 0.25, size * 0.06, size * 0.15, PALETTE["workbench"])
    # Screen / hologram
    rounded_rect(d, cx - size * 0.15, cy - size * 0.2, size * 0.3, size * 0.2, 2, PALETTE["wb_screen"])
    # Tools on table
    rect(d, cx + size * 0.1, cy + size * 0.05, size * 0.12, size * 0.04, PALETTE["outline"])
    circle(d, cx - size * 0.15, cy + size * 0.1, 4, PALETTE["white"])
    return img


# ---------------------------------------------------------------------------
# Projectile Sprites
# ---------------------------------------------------------------------------

def draw_projectile_bullet(size=16):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    circle(d, cx, cy, size * 0.35, PALETTE["projectile"])
    # Glow center
    circle(d, cx, cy, size * 0.15, PALETTE["white"])
    return img


def draw_projectile_artillery(size=32):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Shell body
    rounded_rect(d, cx - size * 0.35, cy - size * 0.15, size * 0.5, size * 0.3, 3, PALETTE["projectile"])
    # Tip
    polygon(d, [
        (cx + size * 0.15, cy - size * 0.15),
        (cx + size * 0.35, cy),
        (cx + size * 0.15, cy + size * 0.15),
    ], PALETTE["explosion"])
    # Trail
    line(d, cx - size * 0.35, cy, cx - size * 0.45, cy, PALETTE["explosion_light"], width=2)
    return img


def draw_projectile_laser(size=(32, 8)):
    img = new_image(size, square=False)
    d = ImageDraw.Draw(img)
    w, h = size
    # Beam
    rounded_rect(d, 0, 0, w, h, h // 2, PALETTE["laser"])
    # Core
    rounded_rect(d, w * 0.15, h * 0.25, w * 0.7, h * 0.5, h // 4, PALETTE["white"])
    return img


def draw_mining_laser_beam(size=(32, 8)):
    img = new_image(size, square=False)
    d = ImageDraw.Draw(img)
    w, h = size
    # Beam
    rounded_rect(d, 0, 0, w, h, h // 2, PALETTE["mining_laser"])
    # Core
    rounded_rect(d, w * 0.15, h * 0.25, w * 0.7, h * 0.5, h // 4, PALETTE["white"])
    return img


# ---------------------------------------------------------------------------
# Particle Sprites
# ---------------------------------------------------------------------------

def draw_fx_muzzle_flash(size=32):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Starburst
    for angle in [0, 45, 90, 135, 180, 225, 270, 315]:
        rad = math.radians(angle)
        x2 = cx + math.cos(rad) * size * 0.4
        y2 = cy + math.sin(rad) * size * 0.4
        line(d, cx, cy, x2, y2, PALETTE["explosion_light"], width=2)
    # Center
    circle(d, cx, cy, size * 0.15, PALETTE["white"])
    return img


def draw_fx_explosion(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Outer fire
    circle(d, cx, cy, size * 0.35, PALETTE["explosion"])
    # Inner fire
    circle(d, cx, cy, size * 0.22, PALETTE["explosion_light"])
    # Sparks
    for _ in range(6):
        angle = math.radians(_ * 60 + 15)
        x = cx + math.cos(angle) * size * 0.25
        y = cy + math.sin(angle) * size * 0.25
        circle(d, x, y, 2, PALETTE["white"])
    # Center
    circle(d, cx, cy, size * 0.1, PALETTE["white"])
    return img


def draw_fx_spark(size=8):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    circle(d, cx, cy, size * 0.35, PALETTE["spark"])
    return img


def draw_fx_fire_dot(size=16):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Flame shape
    polygon(d, [
        (cx, cy - size * 0.4),
        (cx + size * 0.25, cy + size * 0.2),
        (cx, cy + size * 0.35),
        (cx - size * 0.25, cy + size * 0.2),
    ], PALETTE["fire"])
    # Inner
    polygon(d, [
        (cx, cy - size * 0.15),
        (cx + size * 0.1, cy + size * 0.1),
        (cx, cy + size * 0.2),
        (cx - size * 0.1, cy + size * 0.1),
    ], PALETTE["fire_light"])
    return img


def draw_fx_smoke_puff(size=32):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Multiple overlapping circles for soft smoke
    circle(d, cx - size * 0.1, cy - size * 0.1, size * 0.25, PALETTE["smoke"])
    circle(d, cx + size * 0.1, cy + size * 0.05, size * 0.2, PALETTE["smoke"])
    circle(d, cx, cy + size * 0.1, size * 0.22, PALETTE["smoke"])
    return img


def draw_fx_warp_ring(size=64):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Ring
    for r in [size * 0.35, size * 0.3, size * 0.25]:
        circle(d, cx, cy, r, None, outline=PALETTE["warp"], width=2)
    # Spokes
    for angle in [0, 60, 120, 180, 240, 300]:
        rad = math.radians(angle)
        x1 = cx + math.cos(rad) * size * 0.15
        y1 = cy + math.sin(rad) * size * 0.15
        x2 = cx + math.cos(rad) * size * 0.35
        y2 = cy + math.sin(rad) * size * 0.35
        line(d, x1, y1, x2, y2, PALETTE["warp"], width=1)
    # Center dot
    circle(d, cx, cy, 3, PALETTE["warp"])
    return img


def draw_fx_hit_spark(size=16):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    # Burst lines
    for angle in [0, 90, 180, 270]:
        rad = math.radians(angle)
        x2 = cx + math.cos(rad) * size * 0.4
        y2 = cy + math.sin(rad) * size * 0.4
        line(d, cx, cy, x2, y2, PALETTE["white"], width=2)
    # Center flash
    circle(d, cx, cy, size * 0.2, PALETTE["spark"])
    return img


def draw_fx_mining_spark(size=8):
    img = new_image(size)
    d = ImageDraw.Draw(img)
    cx, cy = size // 2, size // 2
    circle(d, cx, cy, size * 0.35, PALETTE["mining_laser"])
    return img


# ---------------------------------------------------------------------------
# Sprite Registry
# ---------------------------------------------------------------------------

SPRITE_REGISTRY = {
    "entities": {
        "player": (draw_player, 64),
        "rover": (draw_rover, 64),
        "drone": (draw_drone, 64),
        "assault_walker": (draw_assault_walker, 64),
        "sniper_walker": (draw_sniper_walker, 64),
        "heavy_flamethrower": (draw_heavy_flamethrower, 64),
        "siege_unit": (draw_siege_unit, 64),
        "core_warden": (draw_core_warden, 64),
        "trader": (draw_trader, 64),
        "item_drop": (draw_item_drop, 32),
    },
    "resources": {
        "node_iron": (draw_node_iron, 64),
        "node_copper": (draw_node_copper, 64),
        "node_silica": (draw_node_silica, 64),
        "node_uranium": (draw_node_uranium, 64),
        "node_plutonium": (draw_node_plutonium, 64),
        "node_biomass": (draw_node_biomass, 64),
    },
    "structures": {
        "wall_segment": (draw_wall_segment, 64),
        "door": (draw_door, 64),
        "generator": (draw_generator, 64),
        "workbench": (draw_workbench, 64),
    },
    "projectiles": {
        "projectile_bullet": (draw_projectile_bullet, 16),
        "projectile_artillery": (draw_projectile_artillery, 32),
        "projectile_laser": (draw_projectile_laser, (32, 8)),
        "mining_laser_beam": (draw_mining_laser_beam, (32, 8)),
    },
    "particles": {
        "fx_muzzle_flash": (draw_fx_muzzle_flash, 32),
        "fx_explosion": (draw_fx_explosion, 64),
        "fx_spark": (draw_fx_spark, 8),
        "fx_fire_dot": (draw_fx_fire_dot, 16),
        "fx_smoke_puff": (draw_fx_smoke_puff, 32),
        "fx_warp_ring": (draw_fx_warp_ring, 64),
        "fx_hit_spark": (draw_fx_hit_spark, 16),
        "fx_mining_spark": (draw_fx_mining_spark, 8),
    },
}


def generate_sprite(category: str, key: str):
    """Generate a single sprite and save it."""
    if category not in SPRITE_REGISTRY or key not in SPRITE_REGISTRY[category]:
        print(f"[SKIP] Unknown sprite: {category}/{key}")
        return False

    func, size = SPRITE_REGISTRY[category][key]
    img = func(size)
    img = trim_transparent_bounds(img)

    out_dir = OUT_DIR / category
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / f"{key}.png"
    img.save(out_path)
    print(f"  -> {out_path}")
    return True


def generate_all():
    """Generate every sprite in the registry."""
    total = 0
    for category, sprites in SPRITE_REGISTRY.items():
        print(f"[GEN] Category: {category} ({len(sprites)} sprites)")
        for key in sprites:
            if generate_sprite(category, key):
                total += 1
    print(f"\nDone. Generated: {total} sprites.")
    return total


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--all", action="store_true", help="Generate all sprites")
    parser.add_argument("--category", type=str, help="Category to generate")
    parser.add_argument("--key", type=str, help="Specific sprite key")
    args = parser.parse_args()

    if args.all:
        generate_all()
    elif args.category and args.key:
        generate_sprite(args.category, args.key)
    elif args.category:
        if args.category in SPRITE_REGISTRY:
            for key in SPRITE_REGISTRY[args.category]:
                generate_sprite(args.category, key)
        else:
            print(f"Unknown category: {args.category}")
    else:
        print("Use --all, --category, or --category + --key")


if __name__ == "__main__":
    main()
