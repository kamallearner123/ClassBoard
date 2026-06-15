sed -i '402,411c\
    let shape_tools: &[(&str, Tool, &str)] = &[\
        ("╱ Line",    Tool::Line,      "Straight line"),\
        ("□ Rect",    Tool::Rectangle, "Rectangle"),\
        ("○ Circle",  Tool::Circle,    "Circle / Ellipse"),\
        ("→ Arrow",   Tool::Arrow,     "Arrow"),\
        ("★ Star",    Tool::Star,      "5-point star"),\
        ("♥ Heart",   Tool::Heart,     "Heart"),\
        ("△ Triangle",Tool::Triangle,  "Triangle"),\
        ("◇ Diamond", Tool::Diamond,   "Diamond"),\
    ];\
' src/main.rs
