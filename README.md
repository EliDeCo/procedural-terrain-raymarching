# Procedural Terrain Raymarching

![Current Project State](https://github.com/EliDeCo/procedural-terrain-raymarching/blob/main/media/Smooth_shading.png)

![Current Project State](https://github.com/EliDeCo/procedural-terrain-raymarching/blob/main/media/Smooth_Shadows.PNG)

---

## Overview

This repository implements a custom rendering pipeline for heightmap-based terrain using raymarching rather than traditional mesh rasterization.

The primary goals of this project are:
- Explore the performance characteristics of raymarching
- Implement realistic lighting features in a raymarched terrain context
- Benchmark and document optimization strategies

---

## Rendering Architecture

High-level pipeline:
1. Ray setup per pixel in fragment shader
2. Raymarch algorithm based on 2d voxel traversal through a heightmap
4. Intersection position and normal derived from ray-plane intersection
5. Lighting evaluation (diffuse + shadowing) 

---

## Tech Stack

| | |
|---|---|
| Language | Rust |
| Engine | Bevy |
| Shader Language | WGSL |
| Rendering Approach | GPU heightmap raymarching |

---

## Core Milestones

- [x] Heightmap intersection with Lambertian diffuse shading
- [x] Smooth normal computation
- [X] Hard shadows
- [X] Soft shadows



## Possible future enhancements
- [ ] Exponential fog
- [ ] Basic sky model
- [ ] Specular highlights
- [ ] Planar water reflections
- [ ] Ambient occlusion
- [ ] Volumetric cloud layer (procedural noise)
- [ ] Improved terrain generation techniques

---

## Performance Goals

- Real-time performance at 1920x1080
- Maintain reasonable performance on non specialized hardware

Both of these goals have been acheived, as even the low powered laptop retains close to 100 frames per second, with a midrange gaming PC pushing past 600. If this project is revisited to add more of the future enhancements, these goals will remain as the target for performance.

---

## Motivation

Terrain lighting effects like soft shadows, ambient occlusion, and volumetric scattering require sampling the scene at an extreme number points — something that maps naturally onto a raymarcher while requiring extra work in a rasterized pipeline. This project explores the tradeoffs between different rendering methods when it comes to performance and implimentation clarity.

---

## Background

This is the successor to [Planet Simulator](https://github.com/EliDeCo/Planet-Simulator), which used a chunk-based rasterized quadsphere. After hitting performance and visual quality limits — particularly around chunk stitching and atmospheric lighting — the project was rebuilt around a raymarching approach.

## Getting Started

To build and run the project locally, you must have cargo and rust installed.

Clone the repository and run the project in release mode:
```bash
git clone https://github.com/EliDeCo/procedural-terrain-raymarching.git
cd Procedural-Terrain-Raymarching
cargo run --release
```
