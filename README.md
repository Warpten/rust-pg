## What works

- MSAA
- Vertex buffers
- Index buffers
- egui integration

## What doesn't work

- Everything else

## Known issues

> [!CAUTION]
> Intel drivers will crash while allocating descriptor set pools if no pool has to be created.
> https://github.com/Warpten/rust-pg/issues/1

## Credits

- https://github.com/MatchaChoco010/egui-winit-ash-integration
  Shamelessly stolen and adapted to more recent versions of winit and the way I create structures.
