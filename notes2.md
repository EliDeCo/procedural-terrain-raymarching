### A more straitforward plan
Since the last one became an unorganized idea list

#### Actively Working on
- Replicate, diagnose, and fix rendering bugs that occurs for certain render distances

#### Next up

- Max height Mipmap Acceleration Structure
- Day/Night Cycle
- Menu for switching between sky models/tonemapping, setting time of day, etc.

#### On the way
- Procedural Sky (allow switching between approaches to compare)
    1. Homemade Cheap Approximation (Sun angle gradient + sunrise/sunset tint)
    2. Perez All-Weather Sky Model
    3. Preetham Model
    4. Hosek-Wilkie Model
    5. Nishita atmospheric scattering
    6. Bruneton & Neyret precomputed atmospheric scattering
    7. Hillaire
- Approximate global illumination with sky samples + Amient Occlusion
- Specular Highlights
- Volumetric Clouds
- Water
- Improve terrain generation
- Portals

#### Optimizations I could do later
- Optimize Barycentric math for axis aligned triangles
- Keep main thread free when doing initial terrain generation