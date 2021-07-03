# How to Cast Spells

Spellcasting is done by manipulating the flow of energy. A magic wand has a magically "dense"
material at the tip of it, and waving it around pulls and tugs on the magical energy all around us
the same way water can be moved with an oar, or how iron filings can be arranged with a magnet 
(like in a [Wooly Willy](https://www.playmonster.com/product/original-wooly-willy/)).

Mechanically, you draw a series of patterns using your mouse on a hex grid. Each pattern has a different
type, like a direction, creature, spell... While you're drawing, there's bullet time in the background
so you have time to draw.

Patterns are saved left to right as you draw them, on a sort of stack. Some patterns take other patterns; they scan from
right to left for the right types. For example, there might be a "raycast" pattern, that takes a position
to raycast from and direction to cast the ray in, and returns the point at which it hits something. To use it,
you would draw:

> Caster Location
> A direction
> Raycast

Raycast scans from right-to-left (or bottom-to-top in this case), finds the direction and location,
and pops them off the "stack". Then, it pushes the point that direction intersects with a wall.
These could be out-of-order; it just scans for any types as it finds them.

After that, some other hypothetical "explosion" pattern that takes a point could be drawn:

> Raycast(direction, location) -> point
> Explosion

Explosion would scan for the point, pop it, and make an explosion there. Then the spell-drawing would end
because the spell stack is empty.
