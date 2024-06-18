#version 450

// Draw up to 64 chunks
layout (binding = 0) buffer PositionBlock {
    vec3 chunkOrigins[64];
}
layout (binding = 1) buffer HeightMap {
    float value[145];
} heightMap;

const float TileSize = 1600.0f / 3.0f;
const float ChunkSize = TileSize / 16.0f;
const float UnitSize = ChunkSize / 8.0f;

vec2i euclid_div(int b, int d) { // (b % n, b / n) integer division
    int n = floor(b / d);
    int r = b - d * n;
    return vec2i(r, n);
}

void main() {
    // Strip format:
    // 0    1    2    3    4    5    6    7    8
    //    9   10   11   12   13   14   15   16
    // Treated as linear.
    // 0    1    2    3    4    5    6    7    8   9   10   11   12   13   14   15   16
    // X offset between 8 and 9 is half of what it is for every other interval

    // Build x and y into a strip (where a strip is 9 + 8 vertices)
    vec2i strip = eclid_div(gl_VertexID, 17); // (x: vertex ID in strip, y: strip index)

    // Convert to chunk-local world coordinates
    int stripInterval = floor(strip.x / 9);
    vec3 worldPosition = vec3(
        chunkOrigins[gl_InstanceID].x - (strip.x     - 8.5 * stripInterval) * UnitSize,
        chunkOrigins[gl_InstanceID].y - (strip.y * 2 +       stripInterval) * UnitSize,
        chunkOrigins[gl_InstanceID].z + heightMap.value[gl_VertexID]
    );
}
