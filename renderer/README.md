## `renderer crate`

:warning: This particular file is a brain dump more than anything. You shouldn't really read this.

### Render graph
#### Identification

Every object that is part of the graph can be identified by a simple type that should satisfy `Eq`, `PartialEq`, `Ord`, `PartialOrd`, `Hash`, and `Display`.

#### Render passes

Each unit of GPU work is a render pass. Strictly speaking, this is a virtual pass; it does not necessarily correlate to Vulkan's concept of a render pass.
A pass has an assigned name in the graph and is not registered to the graph until the `register` method is called. Passes follow a builder-style API.

```rust
let a : PassID = Pass::new("Pass A")
    .add_input(...)
    .add_output(...)
    .register(&mut graph);
```

#### Resources

There exist three (for now) types of resources in this crate: a buffer, a texture, and a virtual texture. All these are concrete types; however they are part
of a sum `Resource` type. Buffer and texture should be self-explanatory; they map more-or-less 1-to-1 to Vulkan's concepts.

Virtual resources are resources that are tied to a pass. They are mostly used to identify implicit graph dependencies by being exposed as outputs for a pass.

```rust
let framebuffer : ResourceID = ...;

let a : PassID = Pass::new("Pass A").
    .add_output(framebuffer)
    .register(&mut graph);

let pass_a : &Pass = graph.find_pass(a).unwrap();
let a_inputs : Vec<ResourceID> = pass_a.inputs();
```

In the example above (whether it makes sense is another topic I will not cover), `a_inputs` will contain a single `ResourceID` that
will conceptually be equal to `ResourceID::Virtual(a, framebuffer)`. It can then be used as the input for another pass:

```rust
let tex_1 = ...;
let tex_2 = ...;

let a = Pass::new("Pass A")
    .add_input("Input[A]", tex_1)
    .add_output("Output[A]", tex_2)
    .register(&mut graph);

let b = Pass::new("Pass B")
    .add_input("Input[B]", a.output(&graph, "Output[A]")) // (1)
    .add_input("Input[B]", ResourceID::Virtual(a, tex_2)) // (2)
    .register(&mut graph);
```

In this example, both declarations for pass B's input are equivalent; however the first version ensures that the resource actually exists.
Note that we cannot sequence passes without inputs or outputs; the graph is in charge of ordering and synchronization.

The important rule is: **Input and output names should be unique per pass**
