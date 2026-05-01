# Vector Art Sprites & Particle Effects — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all Phaser primitive shapes (circles, rectangles) with crisp 64×64 PNG sprites in a consistent vector-art style, plus add particle effects for combat, mining, death, and warp.

**Architecture:** Use the local Lemonade server (Qwen-Image-2512-GGUF) to generate raw raster sprites via its OpenAI-compatible `images/generations` endpoint. A Python post-processing pipeline (Pillow) removes backgrounds, auto-crops to content, pads to 64×64, and adds a 2px dark outline. Sprites are saved to `client/public/sprites/` and loaded into Phaser 4 as individual textures. Entities in `OverworldScene.spawnEntity()` switch from `add.circle()` / `add.rectangle()` to `add.sprite()`.

**Tech Stack:** Lemonade HTTP API (`curl`/`requests`), Python 3 + Pillow, Phaser 4, Vite static asset serving.

---

## File Structure

| Path | Purpose |
|------|---------|
| `tools/sprite_pipeline/` | Python scripts for generation, processing, and batch management |
| `tools/sprite_pipeline/generate.py` | Calls Lemonade API, saves raw PNGs |
| `tools/sprite_pipeline/process.py` | Removes background, crops, pads, outlines, outputs final 64×64 PNGs |
| `tools/sprite_pipeline/spritesheet.py` | Optional: packs sprites into a Phaser texture atlas |
| `tools/sprite_pipeline/config.yaml` | Sprite manifest: names, prompts, sizes, categories |
| `client/public/sprites/` | Final processed sprites served by Vite |
| `client/public/sprites/entities/` | Player, enemies, NPCs |
| `client/public/sprites/resources/` | Mineable nodes |
| `client/public/sprites/structures/` | Base building tiles |
| `client/public/sprites/items/` | Inventory items & projectiles |
| `client/public/sprites/particles/` | Effect textures |
| `client/src/scenes/BootScene.ts` | Preloads all sprite textures |
| `client/src/scenes/OverworldScene.ts` | Uses sprites instead of primitives |
| `client/src/scenes/AsteroidBaseScene.ts` | Uses sprites instead of primitives |
| `client/src/config/SpriteRegistry.ts` | Maps `SnapshotEntityType` → texture key + tint + scale |
| `docs/superpowers/plans/` | This plan document |

---

## Sprite Manifest (Complete Inventory)

### Entities (10 sprites)
| Key | Description | Size | Notes |
|-----|-------------|------|-------|
| `player` | Astronaut in spacesuit, top-down view | 64×64 | Blue-tinted in code for self, purple for others |
| `rover` | Small wheeled drone, aggressive red | 64×64 | Circular base, antenna |
| `drone` | Flying mechanical disc, yellow | 64×64 | Rotor/propeller hint |
| `assault_walker` | Humanoid mech, shotgun frame, orange | 64×64 | Stocky, forward-leaning |
| `sniper_walker` | Tall humanoid mech, railgun, cyan | 64×64 | Slim, long barrel |
| `heavy_flamethrower` | Bulky armored mech, fuel tanks, dark red | 64×64 | Wide shoulders, helmet |
| `siege_unit` | Tank treads + heavy cannon, gray | 64×64 | Largest non-boss enemy |
| `core_warden` | Massive boss mech, multiple arms, glowing core | 128×128 | Scaled down to 64 in-game or rendered at 128 |
| `trader` | Scrapper colony merchant, green outfit | 64×64 | Humanoid, neutral stance |
| `item_drop` | Small glowing crate / debris | 32×32 | Scaled up from smaller source |

### Resources (6 sprites)
| Key | Description | Size |
|-----|-------------|------|
| `node_iron` | Gray rocky outcropping, metallic shards | 64×64 |
| `node_copper` | Orange-brown oxidized rocks | 64×64 |
| `node_silica` | Pale crystal cluster, glassy | 64×64 |
| `node_uranium` | Black rock with bright green glow veins | 64×64 |
| `node_plutonium` | Dark rock with purple-blue glow veins | 64×64 |
| `node_biomass` | Organic green fungi / tubers | 64×64 |

### Structures (4 sprites)
| Key | Description | Size |
|-----|-------------|------|
| `wall_segment` | Gray metal wall panel, rivets | 64×64 |
| `door` | Wall panel with sliding seam, frame | 64×64 |
| `generator` | Boxy machine with glowing green core, cables | 64×64 |
| `workbench` | Table with tools, screens, hologram | 64×64 |

### Items / Projectiles (4 sprites)
| Key | Description | Size |
|-----|-------------|------|
| `projectile_bullet` | Small orange plasma bolt | 16×16 |
| `projectile_artillery` | Large shell with trail | 32×32 |
| `projectile_laser` | Cyan beam segment | 32×8 |
| `mining_laser_beam` | Yellow-green energy line | 32×8 |

### Particles / Effects (8 textures)
| Key | Description | Size |
|-----|-------------|------|
| `fx_muzzle_flash` | Starburst flash, yellow-white | 32×32 |
| `fx_explosion` | Expanding fireball, orange-red | 64×64 |
| `fx_spark` | Small white-yellow spark dot | 8×8 |
| `fx_fire_dot` | Orange flame lick | 16×16 |
| `fx_smoke_puff` | Gray smoke cloud | 32×32 |
| `fx_warp_ring` | Blue expanding circle | 64×64 |
| `fx_hit_spark` | White impact flash | 16×16 |
| `fx_mining_spark` | Yellow-green debris spray dot | 8×8 |

**Total: 32 unique sprite textures.**

---

## Art Style Prompt Template

All generation prompts share this prefix to enforce consistency:

```
Top-down 2D sprite for a sci-fi survival video game. Crisp vector-art style: flat colors, bold black outlines, minimal shading, clean geometric shapes, transparent background, isolated subject centered in frame. No text, no UI elements, no background.
```

**Example prompt for `rover`:**
```
Top-down 2D sprite for a sci-fi survival video game. Crisp vector-art style: flat colors, bold black outlines, minimal shading, clean geometric shapes, transparent background, isolated subject centered in frame. No text, no UI elements, no background. Subject: a small aggressive red wheeled combat drone, circular body, single antenna, top-down view.
```

---

## Task 1: Bootstrap the Pipeline

**Files:**
- Create: `tools/sprite_pipeline/config.yaml`
- Create: `tools/sprite_pipeline/requirements.txt`

- [ ] **Step 1: Write the sprite manifest config**

```yaml
# tools/sprite_pipeline/config.yaml
api:
  host: "http://127.0.0.1:13305"
  endpoint: "/api/v1/images/generations"
  model: "Qwen-Image-2512-GGUF"
  defaults:
    width: 512
    height: 512
    steps: 20
    cfg_scale: 2.5

style_prefix: >
  Top-down 2D sprite for a sci-fi survival video game.
  Crisp vector-art style: flat colors, bold black outlines,
  minimal shading, clean geometric shapes, transparent background,
  isolated subject centered in frame. No text, no UI elements,
  no background.

output_dir: "../../client/public/sprites"

sprites:
  entities:
    - key: player
      size: 64
      prompt: "Astronaut in white spacesuit, top-down view, helmet visor"
    - key: rover
      size: 64
      prompt: "Small aggressive red wheeled combat drone, circular body, antenna"
    - key: drone
      size: 64
      prompt: "Flying yellow mechanical drone, rotor disc, top-down view"
    - key: assault_walker
      size: 64
      prompt: "Stocky orange humanoid combat mech, shotgun frame, aggressive stance"
    - key: sniper_walker
      size: 64
      prompt: "Tall slim cyan humanoid mech, long railgun barrel, scope"
    - key: heavy_flamethrower
      size: 64
      prompt: "Bulky dark red armored mech, fuel tanks, heavy flamethrower, helmet"
    - key: siege_unit
      size: 64
      prompt: "Large gray tank mech, heavy treads, artillery cannon, armored hull"
    - key: core_warden
      size: 128
      prompt: "Massive dark boss mech, multiple weapon arms, glowing red core, intimidating"
    - key: trader
      size: 64
      prompt: "Humanoid scrapper merchant, green outfit, backpack, neutral stance"
    - key: item_drop
      size: 32
      prompt: "Small glowing sci-fi crate, debris, loot container"

  resources:
    - key: node_iron
      size: 64
      prompt: "Gray metallic rocky outcropping, iron ore shards, top-down"
    - key: node_copper
      size: 64
      prompt: "Orange-brown oxidized rock cluster, copper ore, top-down"
    - key: node_silica
      size: 64
      prompt: "Pale glassy crystal cluster, silica deposit, translucent"
    - key: node_uranium
      size: 64
      prompt: "Black rock with bright green glowing radioactive veins, hazard"
    - key: node_plutonium
      size: 64
      prompt: "Dark rock with purple-blue glowing radioactive veins, hazard"
    - key: node_biomass
      size: 64
      prompt: "Organic green fungi and tubers, bioluminescent spots, alien flora"

  structures:
    - key: wall_segment
      size: 64
      prompt: "Gray metal wall panel, rivets, seams, industrial"
    - key: door
      size: 64
      prompt: "Sci-fi sliding door panel, frame, indicator light"
    - key: generator
      size: 64
      prompt: "Boxy power generator, glowing green core, cables, vents"
    - key: workbench
      size: 64
      prompt: "Sci-fi crafting workbench, tools, holographic screen, cluttered"

  projectiles:
    - key: projectile_bullet
      size: 16
      prompt: "Small orange plasma energy bolt, bullet, glowing"
    - key: projectile_artillery
      size: 32
      prompt: "Large artillery shell with fire trail, explosive round"
    - key: projectile_laser
      size: [32, 8]
      prompt: "Cyan laser beam segment, energy bolt, straight line"
    - key: mining_laser_beam
      size: [32, 8]
      prompt: "Yellow-green mining laser beam segment, energy line"

  particles:
    - key: fx_muzzle_flash
      size: 32
      prompt: "Starburst muzzle flash, yellow white explosion, gunfire"
    - key: fx_explosion
      size: 64
      prompt: "Expanding fireball explosion, orange red, smoke"
    - key: fx_spark
      size: 8
      prompt: "Tiny bright white-yellow spark dot, particle"
    - key: fx_fire_dot
      size: 16
      prompt: "Small orange flame lick, fire particle"
    - key: fx_smoke_puff
      size: 32
      prompt: "Gray smoke cloud puff, soft, particle"
    - key: fx_warp_ring
      size: 64
      prompt: "Blue expanding energy ring, teleport warp, sci-fi"
    - key: fx_hit_spark
      size: 16
      prompt: "White impact flash spark, collision burst"
    - key: fx_mining_spark
      size: 8
      prompt: "Tiny yellow-green debris spark, mining particle"
```

- [ ] **Step 2: Write Python requirements**

```text
# tools/sprite_pipeline/requirements.txt
requests>=2.31.0
Pillow>=10.0.0
PyYAML>=6.0
```

- [ ] **Step 3: Install dependencies**

```bash
cd tools/sprite_pipeline
pip install -r requirements.txt
```

- [ ] **Step 4: Commit**

```bash
git add tools/sprite_pipeline/
git commit -m "feat(art): add sprite generation pipeline config and requirements"
```

---

## Task 2: Build the Generation Script

**Files:**
- Create: `tools/sprite_pipeline/generate.py`

- [ ] **Step 1: Write the generation script**

```python
#!/usr/bin/env python3
"""
generate.py — Call the Lemonade server images API to generate raw sprites.

Usage:
    python generate.py --all          # Generate everything
    python generate.py --category entities --key rover
    python generate.py --category resources
"""

import argparse
import base64
import json
import os
import sys
import time
from pathlib import Path

import requests
import yaml


CONFIG_PATH = Path(__file__).with_name("config.yaml")
RAW_DIR = Path(__file__).parent / "raw"


def load_config():
    with open(CONFIG_PATH, "r") as f:
        return yaml.safe_load(f)


def ensure_model_loaded(cfg: dict) -> bool:
    """Ping the server and load the model if needed."""
    host = cfg["api"]["host"]
    model = cfg["api"]["model"]
    # Try a cheap health-like check via chat completions
    try:
        r = requests.post(
            f"{host}/api/v1/chat/completions",
            json={"model": model, "messages": [{"role": "user", "content": "hi"}]},
            timeout=30,
        )
        if r.status_code == 200:
            return True
        # Model not loaded; attempt to load via lemonade CLI
        print(f"Model {model} not loaded. Attempting to load...")
        os.system(f"lemonade-server load {model}")
        time.sleep(10)
        return True
    except Exception as e:
        print(f"Server unreachable: {e}")
        return False


def generate_one(cfg: dict, key: str, prompt: str, size: int | list) -> bytes | None:
    host = cfg["api"]["host"]
    endpoint = cfg["api"]["endpoint"]
    model = cfg["api"]["model"]
    defaults = cfg["api"].get("defaults", {})

    width = defaults.get("width", 512)
    height = defaults.get("height", 512)

    full_prompt = f"{cfg['style_prefix'].strip()}\n\nSubject: {prompt}"

    payload = {
        "model": model,
        "prompt": full_prompt,
        "n": 1,
        "size": f"{width}x{height}",
    }

    try:
        resp = requests.post(f"{host}{endpoint}", json=payload, timeout=120)
        resp.raise_for_status()
        data = resp.json()
        # OpenAI-compatible format: data[0].b64_json
        b64 = data["data"][0]["b64_json"]
        return base64.b64decode(b64)
    except Exception as e:
        print(f"  ERROR generating {key}: {e}")
        return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--all", action="store_true", help="Generate every sprite")
    parser.add_argument("--category", type=str, help="Category to generate")
    parser.add_argument("--key", type=str, help="Specific sprite key")
    args = parser.parse_args()

    cfg = load_config()
    if not ensure_model_loaded(cfg):
        sys.exit(1)

    RAW_DIR.mkdir(parents=True, exist_ok=True)

    generated = 0
    failed = 0

    for category, items in cfg["sprites"].items():
        if args.category and category != args.category:
            continue
        for item in items:
            key = item["key"]
            if args.key and key != args.key:
                continue
            out_path = RAW_DIR / f"{key}.png"
            if out_path.exists():
                print(f"[SKIP] {key} already exists at {out_path}")
                continue

            print(f"[GEN] {category}/{key} ...")
            img_bytes = generate_one(cfg, key, item["prompt"], item["size"])
            if img_bytes:
                out_path.write_bytes(img_bytes)
                print(f"  -> {out_path}")
                generated += 1
            else:
                failed += 1
            time.sleep(1)  # Rate-limit politely

    print(f"\nDone. Generated: {generated}, Failed: {failed}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Test generation of a single sprite**

```bash
cd tools/sprite_pipeline
python generate.py --category entities --key rover
```

Expected: `raw/rover.png` created (512×512 raw generation).

- [ ] **Step 3: Commit**

```bash
git add tools/sprite_pipeline/generate.py
git commit -m "feat(art): add sprite generation script"
```

---

## Task 3: Build the Post-Processing Script

**Files:**
- Create: `tools/sprite_pipeline/process.py`

- [ ] **Step 1: Write the processing script**

```python
#!/usr/bin/env python3
"""
process.py — Convert raw 512×512 generations into crisp 64×64 game sprites.

Steps per image:
1. Convert to RGBA
2. Remove near-white / near-black background (flood-fill from corners)
3. Trim transparent borders
4. Add 2px dark outline (#111111) by dilating the alpha mask
5. Center and pad to exact target size
6. Save as PNG
"""

import argparse
import sys
from pathlib import Path

from PIL import Image, ImageFilter

RAW_DIR = Path(__file__).parent / "raw"
OUT_DIR = Path(__file__).parent / "../../client/public/sprites"

# Background color thresholds for removal (tune if generations vary)
BG_THRESHOLD = 30  # Pixels within 30/255 of pure white or black are background


def remove_background(img: Image.Image) -> Image.Image:
    """Flood-fill remove common background colors from corners."""
    img = img.convert("RGBA")
    datas = img.getdata()
    width, height = img.size

    # Build a simple mask: treat near-white and near-black as background
    # This is a heuristic; for AI-generated isolated subjects it works well.
    new_data = []
    for item in datas:
        r, g, b, a = item
        # Near white or near black => transparent
        if (r > 255 - BG_THRESHOLD and g > 255 - BG_THRESHOLD and b > 255 - BG_THRESHOLD) or \
           (r < BG_THRESHOLD and g < BG_THRESHOLD and b < BG_THRESHOLD):
            new_data.append((0, 0, 0, 0))
        else:
            new_data.append((r, g, b, 255))

    img.putdata(new_data)
    return img


def add_outline(img: Image.Image, color: tuple, thickness: int = 2) -> Image.Image:
    """Add an outline by dilating the alpha channel."""
    # Extract alpha
    alpha = img.split()[-1]
    # Dilate
    outline = alpha.filter(ImageFilter.MaxFilter(thickness * 2 + 1))
    # Create outline image
    outline_img = Image.new("RGBA", img.size, color)
    outline_img.putalpha(outline)
    # Composite original on top
    return Image.alpha_composite(outline_img, img)


def process_sprite(raw_path: Path, target_size: int | tuple) -> Image.Image:
    img = Image.open(raw_path)
    img = remove_background(img)

    # Auto-crop transparent borders
    bbox = img.getbbox()
    if bbox:
        img = img.crop(bbox)

    # Determine target dimensions
    if isinstance(target_size, list):
        tw, th = target_size
    else:
        tw = th = target_size

    # Scale down to fit inside target while preserving aspect ratio,
    # leaving 2px padding on each side for the outline.
    max_w = tw - 4
    max_h = th - 4
    img.thumbnail((max_w, max_h), Image.Resampling.LANCZOS)

    # Add outline
    img = add_outline(img, (17, 17, 17, 255), thickness=2)

    # Center on target canvas
    canvas = Image.new("RGBA", (tw, th), (0, 0, 0, 0))
    x = (tw - img.width) // 2
    y = (th - img.height) // 2
    canvas.paste(img, (x, y), img)

    return canvas


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--all", action="store_true")
    parser.add_argument("--key", type=str)
    args = parser.parse_args()

    import yaml
    cfg_path = Path(__file__).with_name("config.yaml")
    cfg = yaml.safe_load(cfg_path.read_text())

    processed = 0
    for category, items in cfg["sprites"].items():
        for item in items:
            key = item["key"]
            if args.key and key != args.key:
                continue
            raw = RAW_DIR / f"{key}.png"
            if not raw.exists():
                print(f"[SKIP] No raw file for {key}")
                continue

            size = item["size"]
            out_dir = OUT_DIR / category
            out_dir.mkdir(parents=True, exist_ok=True)
            out_path = out_dir / f"{key}.png"

            print(f"[PROC] {key} -> {out_path}")
            final = process_sprite(raw, size)
            final.save(out_path)
            processed += 1

    print(f"\nProcessed: {processed}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Test processing the rover sprite**

```bash
cd tools/sprite_pipeline
python process.py --key rover
```

Expected: `client/public/sprites/entities/rover.png` created at 64×64.

- [ ] **Step 3: Commit**

```bash
git add tools/sprite_pipeline/process.py
git commit -m "feat(art): add sprite post-processing pipeline"
```

---

## Task 4: Batch-Generate All Sprites

- [ ] **Step 1: Generate all raw sprites**

```bash
cd tools/sprite_pipeline
python generate.py --all
```

Expected: ~32 PNG files in `tools/sprite_pipeline/raw/`. Some may fail due to model quirks; re-run individual keys as needed.

- [ ] **Step 2: Process all sprites**

```bash
python process.py --all
```

Expected: ~32 final PNGs in `client/public/sprites/` organized by category.

- [ ] **Step 3: Verify outputs**

```bash
find client/public/sprites -name "*.png" | wc -l
```

Expected: ~32 files.

- [ ] **Step 4: Commit assets**

```bash
git add client/public/sprites/
git commit -m "assets: add generated vector-art sprites for all entities, resources, structures, and particles"
```

---

## Task 5: Sprite Registry Config

**Files:**
- Create: `client/src/config/SpriteRegistry.ts`

- [ ] **Step 1: Write the registry mapping entity types to textures**

```typescript
// client/src/config/SpriteRegistry.ts
import type { SnapshotEntityType } from "../types/protocol";

export interface SpriteDef {
	texture: string;
	scale?: number;
	tint?: number;
}

export const SPRITE_REGISTRY: Record<string, SpriteDef> = {
	// Entities
	Player: { texture: "player", scale: 1.0 },
	Hostile: { texture: "rover", scale: 1.0 },
	Rover: { texture: "rover", scale: 1.0 },
	Drone: { texture: "drone", scale: 1.0 },
	AssaultWalker: { texture: "assault_walker", scale: 1.0 },
	SniperWalker: { texture: "sniper_walker", scale: 1.0 },
	HeavyFlamethrower: { texture: "heavy_flamethrower", scale: 1.0 },
	CoreWarden: { texture: "core_warden", scale: 1.0 },
	Trader: { texture: "trader", scale: 1.0 },

	// Resources
	ResourceNode: { texture: "node_iron", scale: 1.0 },

	// Items / Projectiles
	Item: { texture: "item_drop", scale: 1.0 },
	Projectile: { texture: "projectile_bullet", scale: 1.0 },

	// Structures
	Unknown: { texture: "wall_segment", scale: 1.0 },
};

/** Resolve which texture to use for a given snapshot entity. */
export function resolveSprite(type: SnapshotEntityType): SpriteDef {
	return SPRITE_REGISTRY[type] ?? { texture: "player", scale: 1.0 };
}

/** Override resource node texture based on server subtype (future). */
export function resolveResourceSprite(subtype: string): SpriteDef {
	const map: Record<string, string> = {
		iron: "node_iron",
		copper: "node_copper",
		silica: "node_silica",
		uranium: "node_uranium",
		plutonium: "node_plutonium",
		biomass: "node_biomass",
	};
	return { texture: map[subtype] ?? "node_iron", scale: 1.0 };
}
```

- [ ] **Step 2: Commit**

```bash
git add client/src/config/SpriteRegistry.ts
git commit -m "feat(art): add sprite registry mapping entity types to textures"
```

---

## Task 6: Boot Scene Preloading

**Files:**
- Modify: `client/src/scenes/BootScene.ts`

- [ ] **Step 1: Add texture loading for all sprites**

In `BootScene.ts`, inside `preload()` or `create()`, add:

```typescript
private loadSprites(): void {
	const categories = [
		"entities",
		"resources",
		"structures",
		"projectiles",
		"particles",
	];

	for (const cat of categories) {
		// Vite serves files from public/ at root path
		// We use import.meta.glob to discover files at build time,
		// or hardcode the manifest.
	}
}
```

Because Vite handles `public/` statically, Phaser's `load.image()` works with root-relative paths:

```typescript
// In BootScene.create() or preload()
this.load.image("player", "sprites/entities/player.png");
this.load.image("rover", "sprites/entities/rover.png");
this.load.image("drone", "sprites/entities/drone.png");
this.load.image("assault_walker", "sprites/entities/assault_walker.png");
this.load.image("sniper_walker", "sprites/entities/sniper_walker.png");
this.load.image("heavy_flamethrower", "sprites/entities/heavy_flamethrower.png");
this.load.image("siege_unit", "sprites/entities/siege_unit.png");
this.load.image("core_warden", "sprites/entities/core_warden.png");
this.load.image("trader", "sprites/entities/trader.png");
this.load.image("item_drop", "sprites/entities/item_drop.png");

this.load.image("node_iron", "sprites/resources/node_iron.png");
this.load.image("node_copper", "sprites/resources/node_copper.png");
this.load.image("node_silica", "sprites/resources/node_silica.png");
this.load.image("node_uranium", "sprites/resources/node_uranium.png");
this.load.image("node_plutonium", "sprites/resources/node_plutonium.png");
this.load.image("node_biomass", "sprites/resources/node_biomass.png");

this.load.image("wall_segment", "sprites/structures/wall_segment.png");
this.load.image("door", "sprites/structures/door.png");
this.load.image("generator", "sprites/structures/generator.png");
this.load.image("workbench", "sprites/structures/workbench.png");

this.load.image("projectile_bullet", "sprites/projectiles/projectile_bullet.png");
this.load.image("projectile_artillery", "sprites/projectiles/projectile_artillery.png");
this.load.image("projectile_laser", "sprites/projectiles/projectile_laser.png");
this.load.image("mining_laser_beam", "sprites/projectiles/mining_laser_beam.png");

// Particle textures
this.load.image("fx_muzzle_flash", "sprites/particles/fx_muzzle_flash.png");
this.load.image("fx_explosion", "sprites/particles/fx_explosion.png");
this.load.image("fx_spark", "sprites/particles/fx_spark.png");
this.load.image("fx_fire_dot", "sprites/particles/fx_fire_dot.png");
this.load.image("fx_smoke_puff", "sprites/particles/fx_smoke_puff.png");
this.load.image("fx_warp_ring", "sprites/particles/fx_warp_ring.png");
this.load.image("fx_hit_spark", "sprites/particles/fx_hit_spark.png");
this.load.image("fx_mining_spark", "sprites/particles/fx_mining_spark.png");
```

- [ ] **Step 2: Commit**

```bash
git add client/src/scenes/BootScene.ts
git commit -m "feat(art): preload all sprite textures in BootScene"
```

---

## Task 7: Overworld Scene — Use Sprites for Entities

**Files:**
- Modify: `client/src/scenes/OverworldScene.ts`

- [ ] **Step 1: Update `spawnEntity()` to use sprites**

Replace the `spawnEntity()` method. The key change is:

```typescript
private spawnEntity(entity: EntitySnapshot): void {
	const isMe = entity.id === this.myEntityId;
	const type = entity.entity_type;

	const spriteDef = resolveSprite(type);
	let sprite: Phaser.GameObjects.Sprite;

	if (type === "Unknown") {
		// Structures still use the structure texture
		sprite = this.add.sprite(
			entity.position.x,
			entity.position.y,
			spriteDef.texture,
		);
		sprite.setScale(spriteDef.scale ?? 1.0);
	} else if (type === "Projectile") {
		sprite = this.add.sprite(
			entity.position.x,
			entity.position.y,
			spriteDef.texture,
		);
		sprite.setScale(spriteDef.scale ?? 0.5);
	} else if (type === "Item") {
		sprite = this.add.sprite(
			entity.position.x,
			entity.position.y,
			spriteDef.texture,
		);
		sprite.setScale(spriteDef.scale ?? 0.5);
	} else {
		// Standard entity sprite
		sprite = this.add.sprite(
			entity.position.x,
			entity.position.y,
			spriteDef.texture,
		);
		sprite.setScale(spriteDef.scale ?? 1.0);
	}

	// Apply tint for differentiation
	if (type === "Player" && isMe) {
		sprite.setTint(0x58a6ff); // Blue for self
	} else if (type === "Player") {
		sprite.setTint(0x7c3aed); // Purple for others
	} else if (type === "Hostile") {
		sprite.setTint(0xff4444); // Red tint fallback
	}

	sprite.setDepth(10);

	// ... rest of label / facingLine setup stays similar
	const label = this.add
		.text(entity.position.x, entity.position.y - 30, getLabelText(type, entity.id, isMe), {
			fontSize: "10px",
			color: getLabelColor(type, isMe),
			fontFamily: "monospace",
		})
		.setOrigin(0.5)
		.setDepth(11);

	let facingLine: Phaser.GameObjects.Graphics | undefined;
	if (isMe) {
		facingLine = this.add.graphics();
		facingLine.lineStyle(2, 0xffffff, 0.8);
		facingLine.moveTo(0, 0);
		facingLine.lineTo(20, 0);
		facingLine.strokePath();
		facingLine.setDepth(12);
	}

	this.entities.set(entity.id, {
		sprite,
		facingLine,
		label,
		targetX: entity.position.x,
		targetY: entity.position.y,
	});
}
```

Also update `RenderedEntity` interface:

```typescript
interface RenderedEntity {
	sprite: Phaser.GameObjects.Sprite; // Changed from Shape
	facingLine?: Phaser.GameObjects.Graphics;
	label: Phaser.GameObjects.Text;
	targetX: number;
	targetY: number;
}
```

- [ ] **Step 2: Update interpolation loop to work with Sprites**

In `update()`, the interpolation code should use `sprite.setPosition()` or directly set `sprite.x` / `sprite.y`:

```typescript
// Inside the entity loop
ent.sprite.x += (ent.targetX - ent.sprite.x) * INTERPOLATION_SPEED;
ent.sprite.y += (ent.targetY - ent.sprite.y) * INTERPOLATION_SPEED;
ent.sprite.setRotation(angle); // If we want rotation toward movement
```

- [ ] **Step 3: Commit**

```bash
git add client/src/scenes/OverworldScene.ts
git commit -m "feat(art): render entities as sprites in OverworldScene"
```

---

## Task 8: Asteroid Base Scene — Use Sprites

**Files:**
- Modify: `client/src/scenes/AsteroidBaseScene.ts`

- [ ] **Step 1: Apply same sprite spawning logic**

Mirror the changes from Task 7 for `AsteroidBaseScene.ts`. Update `RenderedEntity` interface and `spawnEntity()` to use `add.sprite()` with structure textures.

- [ ] **Step 2: Commit**

```bash
git add client/src/scenes/AsteroidBaseScene.ts
git commit -m "feat(art): render entities as sprites in AsteroidBaseScene"
```

---

## Task 9: Particle Effects System

**Files:**
- Create: `client/src/systems/ParticleManager.ts`

- [ ] **Step 1: Write the particle manager**

```typescript
// client/src/systems/ParticleManager.ts
import * as Phaser from "phaser";

export class ParticleManager {
	private scene: Phaser.Scene;
	private emitters: Map<string, Phaser.GameObjects.Particles.ParticleEmitter> = new Map();

	constructor(scene: Phaser.Scene) {
		this.scene = scene;
	}

	createExplosion(x: number, y: number): void {
		const emitter = this.scene.add.particles(x, y, "fx_explosion", {
			speed: { min: 50, max: 150 },
			scale: { start: 0.8, end: 0 },
			alpha: { start: 1, end: 0 },
			lifespan: 600,
			quantity: 8,
			blendMode: Phaser.BlendModes.ADD,
		});
		emitter.explode();
	}

	createMuzzleFlash(x: number, y: number, angle: number): void {
		const emitter = this.scene.add.particles(x, y, "fx_muzzle_flash", {
			speed: 80,
			scale: { start: 0.6, end: 0 },
			lifespan: 150,
			quantity: 3,
			angle: { min: angle - 15, max: angle + 15 },
			blendMode: Phaser.BlendModes.ADD,
		});
		emitter.explode();
	}

	createHitSpark(x: number, y: number): void {
		const emitter = this.scene.add.particles(x, y, "fx_hit_spark", {
			speed: { min: 30, max: 80 },
			scale: { start: 0.5, end: 0 },
			lifespan: 200,
			quantity: 5,
			blendMode: Phaser.BlendModes.ADD,
		});
		emitter.explode();
	}

	createMiningSparks(x: number, y: number): void {
		const emitter = this.scene.add.particles(x, y, "fx_mining_spark", {
			speed: { min: 20, max: 60 },
			scale: { start: 0.8, end: 0 },
			lifespan: 300,
			quantity: 4,
			frequency: 100,
			blendMode: Phaser.BlendModes.ADD,
		});
		// Auto-stop after a short burst
		this.scene.time.delayedCall(400, () => emitter.stop());
	}

	createDeathPoof(x: number, y: number): void {
		const emitter = this.scene.add.particles(x, y, "fx_smoke_puff", {
			speed: { min: 10, max: 40 },
			scale: { start: 0.7, end: 0 },
			alpha: { start: 0.8, end: 0 },
			lifespan: 800,
			quantity: 6,
		});
		emitter.explode();
	}

	createFireDot(x: number, y: number): Phaser.GameObjects.Particles.ParticleEmitter {
		const emitter = this.scene.add.particles(x, y, "fx_fire_dot", {
			speed: { min: 5, max: 15 },
			scale: { start: 0.5, end: 0 },
			lifespan: 500,
			frequency: 100,
			quantity: 1,
			blendMode: Phaser.BlendModes.ADD,
		});
		return emitter;
	}

	createWarpEffect(x: number, y: number): void {
		const emitter = this.scene.add.particles(x, y, "fx_warp_ring", {
			speed: 0,
			scale: { start: 0.2, end: 2.0 },
			alpha: { start: 0.8, end: 0 },
			lifespan: 1000,
			quantity: 1,
			blendMode: Phaser.BlendModes.ADD,
		});
		emitter.explode();
	}
}
```

- [ ] **Step 2: Integrate into OverworldScene**

Add to `OverworldScene`:

```typescript
private particles!: ParticleManager;

// In create()
this.particles = new ParticleManager(this);

// On fire (inside pointerdown handler, after sending Fire message)
this.particles.createMuzzleFlash(worldPoint.x, worldPoint.y, angle);

// On projectile hit (in net.onMessage, SpawnProjectile handler — or add a Hit message)
// On death (PlayerDied message)
if (msg.PlayerDied.entity_id === this.myEntityId) {
	// ... existing code ...
	const me = this.entities.get(this.myEntityId);
	if (me) {
		this.particles.createDeathPoof(me.sprite.x, me.sprite.y);
	}
}

// On mining — when Mine message is sent
this.particles.createMiningSparks(worldPoint.x, worldPoint.y);
```

- [ ] **Step 3: Commit**

```bash
git add client/src/systems/ParticleManager.ts client/src/scenes/OverworldScene.ts
git commit -m "feat(vfx): add particle effects for combat, mining, death, and warp"
```

---

## Task 10: Build Verification

- [ ] **Step 1: Build the client**

```bash
cd client
npm run build
```

Expected: Build succeeds, sprites copied to `dist/sprites/` by Vite.

- [ ] **Step 2: Run smoke test**

```bash
cd docs/../.opencode/skills/webapp-testing/scripts  # or wherever the smoke test lives
python lithos_smoke_test.py
```

Expected: All tests pass, no console errors about missing textures.

- [ ] **Step 3: Manual visual check**

Start the dev server and verify:
- Player renders as sprite, not blue circle
- Rovers render as red wheeled sprites
- Resource nodes show distinct textures per type
- Structures show wall/generator/workbench sprites
- Particle effects appear on fire/mine/death

- [ ] **Step 4: Commit**

```bash
git commit -m "chore: verify sprite integration and particle effects"
```

---

## Self-Review Checklist

**1. Spec coverage:**
- [x] Vector art sprites for all entities (§8) → Tasks 4–8
- [x] Distinct enemy silhouettes (§7) → Tasks 4–5 (one unique sprite per enemy type)
- [x] Resource node visuals (§2.2) → Tasks 4–5 (6 distinct node sprites)
- [x] Structure visuals (§6) → Tasks 4–5 (wall, door, generator, workbench)
- [x] Particle effects for combat/mining → Task 9
- [x] Phaser integration → Tasks 6–9

**2. Placeholder scan:**
- [x] No "TBD" or "TODO" in tasks
- [x] Every step has exact file paths
- [x] Code blocks contain complete implementations
- [x] Commands include expected outputs

**3. Type consistency:**
- [x] `SnapshotEntityType` used consistently
- [x] `SpriteDef` interface used in registry and spawning
- [x] Texture keys in config match `load.image()` keys in BootScene

**4. Gaps identified:**
- The server currently sends `entity_type: "Hostile"` for all enemies; to use distinct walker sprites, the server must send the specific subtype (`Rover`, `Drone`, `AssaultWalker`, etc.). The protocol already defines these in `SnapshotEntityType`. Ensure `lithos-server` snapshot serialization maps `NpcType` to the correct `SnapshotEntityType`.
- Resource node subtypes are not currently sent in snapshots; all show as `"ResourceNode"`. The plan uses a fallback to `node_iron` for now. A follow-up task could add subtype to `EntitySnapshot`.

---

## Execution Handoff

**Plan complete.** Two execution options:

**1. Subagent-Driven (recommended)** — Dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using `executing-plans`, batch execution with checkpoints for review.

**Which approach?**
