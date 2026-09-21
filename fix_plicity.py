import re

with open('src/ast.rs', 'r') as f:
    c = f.read()

# Add Plicity enum
plicity_enum = """
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Plicity {
    Explicit,
    Implicit,
}
"""

if "pub enum Plicity" not in c:
    c = c.replace("pub enum Quantity", plicity_enum + "\n#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum Quantity")

# Add Plicity to Pi, Lambda, App
# Pi { plicity: Plicity, quantity: Quantity, domain: ExprId, codomain: ExprId }
c = re.sub(r'Pi \{\n\s*quantity: Quantity,\n\s*domain: ExprId,\n\s*codomain: ExprId,\n\s*\},', r'Pi {\n        plicity: Plicity,\n        quantity: Quantity,\n        domain: ExprId,\n        codomain: ExprId,\n    },', c)

# Lambda { plicity: Plicity, quantity: Quantity, body: ExprId }
c = re.sub(r'Lambda \{\n\s*quantity: Quantity,\n\s*body: ExprId,\n\s*\},', r'Lambda {\n        plicity: Plicity,\n        quantity: Quantity,\n        body: ExprId,\n    },', c)

# App { plicity: Plicity, function: ExprId, argument: ExprId }
c = re.sub(r'App \{\n\s*function: ExprId,\n\s*argument: ExprId,\n\s*\},', r'App {\n        plicity: Plicity,\n        function: ExprId,\n        argument: ExprId,\n    },', c)

with open('src/ast.rs', 'w') as f:
    f.write(c)

