//! Unity's built-in typetree string table.
//!
//! A typetree node names its type and field either by an offset into the
//! file's own string buffer or, with the high bit set, by an offset into this
//! table, which Unity compiles in and every file shares. It is one run of
//! NUL-terminated names; it only ever grew at the end, so the offsets seen in
//! 2018.3 and 2020.3 files are a prefix of the table below.
//!
//! **Provenance:** the names were read from the table UnityPy 1.25.4 carries
//! (facts, not code), then checked against the content: every high-bit offset in
//! all 111 SerializedFiles of the census lands on a string boundary, and the
//! typetree built from it consumes every object's bytes exactly
//! (`tests/serialized_ground_truth.rs`), which a wrong name would not survive.

/// The table, strings separated by NUL, ending in one.
const TABLE: &str = concat!(
    "AABB\0",
    "AnimationClip\0",
    "AnimationCurve\0",
    "AnimationState\0",
    "Array\0",
    "Base\0",
    "BitField\0",
    "bitset\0",
    "bool\0",
    "char\0",
    "ColorRGBA\0",
    "Component\0",
    "data\0",
    "deque\0",
    "double\0",
    "dynamic_array\0",
    "FastPropertyName\0",
    "first\0",
    "float\0",
    "Font\0",
    "GameObject\0",
    "Generic Mono\0",
    "GradientNEW\0",
    "GUID\0",
    "GUIStyle\0",
    "int\0",
    "list\0",
    "long long\0",
    "map\0",
    "Matrix4x4f\0",
    "MdFour\0",
    "MonoBehaviour\0",
    "MonoScript\0",
    "m_ByteSize\0",
    "m_Curve\0",
    "m_EditorClassIdentifier\0",
    "m_EditorHideFlags\0",
    "m_Enabled\0",
    "m_ExtensionPtr\0",
    "m_GameObject\0",
    "m_Index\0",
    "m_IsArray\0",
    "m_IsStatic\0",
    "m_MetaFlag\0",
    "m_Name\0",
    "m_ObjectHideFlags\0",
    "m_PrefabInternal\0",
    "m_PrefabParentObject\0",
    "m_Script\0",
    "m_StaticEditorFlags\0",
    "m_Type\0",
    "m_Version\0",
    "Object\0",
    "pair\0",
    "PPtr<Component>\0",
    "PPtr<GameObject>\0",
    "PPtr<Material>\0",
    "PPtr<MonoBehaviour>\0",
    "PPtr<MonoScript>\0",
    "PPtr<Object>\0",
    "PPtr<Prefab>\0",
    "PPtr<Sprite>\0",
    "PPtr<TextAsset>\0",
    "PPtr<Texture>\0",
    "PPtr<Texture2D>\0",
    "PPtr<Transform>\0",
    "Prefab\0",
    "Quaternionf\0",
    "Rectf\0",
    "RectInt\0",
    "RectOffset\0",
    "second\0",
    "set\0",
    "short\0",
    "size\0",
    "SInt16\0",
    "SInt32\0",
    "SInt64\0",
    "SInt8\0",
    "staticvector\0",
    "string\0",
    "TextAsset\0",
    "TextMesh\0",
    "Texture\0",
    "Texture2D\0",
    "Transform\0",
    "TypelessData\0",
    "UInt16\0",
    "UInt32\0",
    "UInt64\0",
    "UInt8\0",
    "unsigned int\0",
    "unsigned long long\0",
    "unsigned short\0",
    "vector\0",
    "Vector2f\0",
    "Vector3f\0",
    "Vector4f\0",
    "m_ScriptingClassIdentifier\0",
    "Gradient\0",
    "Type*\0",
    "int2_storage\0",
    "int3_storage\0",
    "BoundsInt\0",
    "m_CorrespondingSourceObject\0",
    "m_PrefabInstance\0",
    "m_PrefabAsset\0",
    "FileSize\0",
    "Hash128\0",
    "RenderingLayerMask\0",
    "fixed_array\0",
    "EntityId\0",
    "LoadableObjectId\0",
    "LoadableSceneId\0",
);

/// The string at `offset` into the table, or `None` when it is not the start of one.
#[must_use]
pub fn common_string(offset: u32) -> Option<&'static str> {
    let o = offset as usize;
    if o >= TABLE.len() || (o > 0 && TABLE.as_bytes()[o - 1] != 0) {
        return None;
    }
    let end = TABLE[o..].find('\0')?;
    Some(&TABLE[o..o + end])
}
