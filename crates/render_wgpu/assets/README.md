# Bundled data

## `tony_mc_mapface_48.rgb9e5`

The TonyMcMapface tone-mapping LUT by Tomasz Stachowiak,
<https://github.com/h3r2tic/tony-mc-mapface> (commit 909e51c), dual licensed under the
MIT license or the Apache License 2.0, at your option.

A 48×48×48 3D texture, `RGB9E5` (`wgpu::TextureFormat::Rgb9e5Ufloat`), 4 bytes per texel,
x fastest, then y, then z; no header. It is the level data of Bevy's
`tony_mc_mapface.ktx2` (`bevy_core_pipeline`, MIT or Apache-2.0), itself converted from
the original `tony_mc_mapface.dds`, with the KTX2 container's Zstandard supercompression
removed so it uploads as is.

Copyright (c) 2023 Tomasz Stachowiak.

Permission is hereby granted, free of charge, to any person obtaining a copy of this
software and associated documentation files (the "Software"), to deal in the Software
without restriction, including without limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons
to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or
substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR
PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
