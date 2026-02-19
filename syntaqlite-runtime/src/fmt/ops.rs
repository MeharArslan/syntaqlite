pub type StringId = u16;
pub type FieldIdx = u16;
pub type SkipCount = u16;

/// A node's formatting entry: bytecode ops.
#[derive(Debug, Clone, Copy)]
pub struct NodeFmt {
    pub ops: &'static [FmtOp],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FmtOp {
    /// Emit a keyword from the string table.
    Keyword(StringId),
    /// Emit source text from a Span field.
    Span(FieldIdx),
    /// Recursively format the child node whose ID is in a NodeId field.
    /// Skipped if the child ID is NULL_NODE.
    Child(FieldIdx),
    /// Flat: space. Break: newline + indent.
    Line,
    /// Flat: empty. Break: newline + indent.
    SoftLine,
    /// Always newline + indent.
    HardLine,
    /// Begin a group (try flat, break if doesn't fit).
    GroupStart,
    /// End a group.
    GroupEnd,
    /// Begin indentation nest.
    NestStart(i16),
    /// End indentation nest.
    NestEnd,
    /// If NodeId field != NULL_NODE, execute next ops; else skip.
    IfSet(FieldIdx, SkipCount),
    /// End of then-branch. If reached, skip the else-branch.
    Else(SkipCount),
    /// No-op marker ending a conditional block.
    EndIf,
    /// Begin iterating children of the list node referenced by a NodeId field.
    ForEachStart(FieldIdx),
    /// Format the current iteration child.
    ChildItem,
    /// Emit separator text between list items (not after last).
    ForEachSep(StringId),
    /// End of ForEach body.
    ForEachEnd,
    /// If Bool field is true, execute next ops; else skip.
    IfBool(FieldIdx, SkipCount),
    /// If Flags field has (value & mask) != 0, execute next ops; else skip.
    IfFlag(FieldIdx, u8, SkipCount),
    /// If Enum field == variant ordinal, execute next ops; else skip.
    IfEnum(FieldIdx, u16, SkipCount),
    /// If Span field is non-empty, execute next ops; else skip.
    IfSpan(FieldIdx, SkipCount),
    /// Map enum ordinal → string via lookup table. `u16` is base index into enum_display table.
    EnumDisplay(FieldIdx, u16),
    /// Begin iterating children of self (for list nodes).
    ForEachSelfStart,
}
