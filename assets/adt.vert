#version 450

// Fix these later
layout (binding = 1) buffer PositionBlock {
    vec3 chunkOrigins[64];
}
layout (binding = 1) heightMap;

const float TileSize = 1600.0f / 3.0f;
const float ChunkSize = TileSize / 16.0f;
const float UnitSize = ChunkSize / 8.0f;

void main() {
    // Strip format:
    // 0    1    2    3    4    5    6    7    8
    //    9   10   11   12   13   14   15   16
    // Treated as linear.
    // 0    1    2    3    4    5    6    7    8   9   10   11   12   13   14   15   16
    // X offset between 8 and 9 is half of what it is for every other interval

    // Build x and y into a strip (where a strip is 9 + 8 vertices)
    vec2i strip = vec2i(gl_VertexID % 17, gl_VertexID / 17); // (x: vertex ID in strip, y: strip index)

    // Convert to chunk-local world coordinates
    gl_Position.x = chunkOrigins[gl_InstanceID].x - (strip.x - 8.5 * floor(strip.x / 9)) * UnitSize;
    gl_Position.y = chunkOrigins[gl_InstanceID].y - (strip.y * 2 + floor(strip.x / 9)) * UnitSize;
    gl_Position.z = chunkOrigins[gl_InstanceID].z + heightMap[gl_VertexID];
}
