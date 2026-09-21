import re

with open('src/ast.rs', 'r') as f:
    c = f.read()

c = c.replace('pub enum Expr {\n    Unit,', '#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub enum Plicity {\n    Explicit,\n    Implicit,\n}\n\n#[derive(Debug, Clone, PartialEq)]\npub enum Expr {\n    Unit,')
c = c.replace('Pi {\n        quantity: Quantity,\n        domain: ExprId,\n        codomain: ExprId,\n    }', 'Pi {\n        plicity: Plicity,\n        quantity: Quantity,\n        domain: ExprId,\n        codomain: ExprId,\n    }')
c = c.replace('Lambda {\n        quantity: Quantity,\n        body: ExprId,\n    }', 'Lambda {\n        plicity: Plicity,\n        quantity: Quantity,\n        body: ExprId,\n    }')
c = c.replace('App {\n        function: ExprId,\n        argument: ExprId,\n    }', 'App {\n        plicity: Plicity,\n        function: ExprId,\n        argument: ExprId,\n    }')

with open('src/ast.rs', 'w') as f:
    f.write(c)
