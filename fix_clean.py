import re

with open('src/eval.rs', 'r') as f:
    c = f.read()

c = re.sub(r'            println!\("Evaluating App \Q{:?}\E", expr\);\n', '', c)
c = re.sub(r'                    println!\("Found \{\} inserted implicits!", implicits\.len\(\)\);\n', '', c)

with open('src/eval.rs', 'w') as f:
    f.write(c)

with open('src/elaborator.rs', 'r') as f:
    c = f.read()

c = re.sub(r'                println!\("Synthesized function type: \Q{:?}\E", f_elab\.ty\);\n', '', c)
c = re.sub(r'                println!\("Application plicity: \Q{:?}\E", plicity\);\n', '', c)
c = re.sub(r'                        println!\("Inserting \{\} implicits for expr \Q{:?}\E", implicits_to_insert\.len\(\), expr\);\n', '', c)

with open('src/elaborator.rs', 'w') as f:
    f.write(c)

