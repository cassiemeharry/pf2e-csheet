# Pathfinder 2 Character Sheet Creator

## Technical Design

A character is structured as a tree of things called "resources". A resource is
anything you'd find in a rule book, like an action, class, feat, or item.

At some point, though, these resources need to translate into numbers for dice
rolls. This is accomplished by adding "effects" to resources. There are a
variety of effects that can be added. The simplest is providing a bonus to some
label, such as "AC", "attack", "weapon damage", "speed", or even basic stats
like "DEX" or "INT". Bonuses are typed as normal. Bonuses can have conditions on
them to limit their scope, such as an "attack" bonus applying only to natural
attacks.

The other common effect is providing another resource, such as taking a feat
that has an associated action or providing a natural attack.

There are several other effects as well, such as providing proficiency in a
skill/attribute/weapon/armor/etc., adding traits to another resource. See [the
resource reference][res-ref] for more details.

Some resources have chocies associated with them. Most choices are local to the
resource, but they can also be associated with another resource where that makes
sense (such as a choice that is made once for an entire class). Within a
resource definition, choices are referred to with a code name starting with `$`.

When referring to other resources that have a choice, the resource name is
written out with the choice appended in parenthesis. For example, there is a
Monk class feature named "incredible movement". The first version, gained at
level 3, grants 10 ft. of bonus movement speed, and is therefore written as
"incredible movement (+10 ft)".

Many resources have a text description to show on the character sheet. These
descriptions can contain calculations within `[[` and `]]` markers. The basic
math operators `+`, `-`, `*`, and `/` are allowed, though precedence between
multiple operators must be specified explicitly with parentheses. The values are
either named labels (such as AC, attack, WIS, etc.), literal numbers, choice
labels (prefixed with `$`), or dice rolls (as MdN). When rendered, the
descriptions will look up the appropriate values for the names and choices, then
evaluate the math where possible.

<a name="res-ref"></a>
## Resource Reference

### Resources

* Class
* Class Feature
* Feat
* Action
* Item

### Effects

* Bonus
* Penalty
* Grant Resource
* 
