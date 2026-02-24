# Procedural Terrain Raymarching

> Real-time 2.5D procedural terrain renderer built in Rust using Bevy and WGSL.

---

## Overview

This repository implements a custom rendering pipeline for heightmap-based terrain using raymarching rather than traditional mesh rasterization.

The primary goals of this project are:
- Explore the performance characteristics of raymarching
- Implement realistic lighting features in a raymarched terrain context
- Benchmark and document optimization strategies

---

## Current Status

- Custom raymarching pipeline integrated into Bevy
- Fragment shader ray setup
- Basic ray direction debugging output

Terrain rendering and lighting features are currently under active development.

---

## Rendering Architecture

High-level pipeline:
1. Ray setup per pixel in fragment shader
2. Raymarch algorithm based on 2d voxel traversal through a heightmap
4. Intersection position and normal derived from ray-plane intersection
5. Lighting evaluation (diffuse + shadowing)
6. Atmospheric and post-surface effects

The renderer operates entirely on the GPU and avoids using meshes.

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

These define the minimum complete renderer:
- [ ] Heightmap intersection with Lambertian diffuse shading
- [ ] Smooth normal computation
- [ ] Hard shadows
- [ ] Exponential fog
- [ ] Basic sky model

---

## Planned Enhancements

After core rendering is stable and benchmarked:
- [ ] Soft shadows
- [ ] Specular highlights
- [ ] Planar water reflections
- [ ] Ambient occlusion
- [ ] Volumetric cloud layer (procedural noise)
- [ ] Improved terrain generation techniques
- [ ] Dynamic render distance based on camera position (if feasible)

---

## Performance Goals

- Real-time performance at 1920x1080
- Maintain reasonable performance on non specialized hardware

---

## Motivation

Terrain lighting effects like soft shadows, ambient occlusion, and volumetric scattering require sampling the scene at arbitrary points — something that maps naturally onto a raymarcher while requiring extra work in a rasterized pipeline

This project explores the tradeoffs between:
- Traditional mesh-based terrain rasterization
- Hybrid raster + raymarch approaches
- Pure heightmap raymarching

The focus is on architectural clarity, performance optimization, and rendering technique evaluation.

---

## Media

Screenshots and benchmarks will be added as terrain rendering and lighting features are completed.

---

## Background

This is the successor to [Planet Simulator](https://github.com/EliDeCo/Planet-Simulator), which used a chunk-based rasterized quadsphere. After hitting performance and visual quality limits — particularly around chunk stitching and atmospheric lighting — the project was rebuilt around a raymarching approach.

