# Arvel Iris

**A visual structure description model.** Takes a graphic — diagram, figure, molecular structure, math equation, schematic, chart — and outputs a structured text description that faithfully represents what's shown. ~60M params, ~62MB on disk, ~55-80ms per image on CPU.

Part of the [Arvel](https://github.com/aldenbernsteinn) family. Lens selects which frames to send. **Iris describes what those frames contain** — so the API LLM doesn't need to see the image at all for ~85% of visual content.

---

## Context: Why This Exists

### The problem
Sending images to API LLMs is expensive. Each image costs ~1,000 tokens. After Arvel Lens selects 21 unique frames from a video, that's still 21,000 tokens just for images.

### The solution
Replace most images with structured text descriptions. A text description of a diagram costs ~20-30 tokens instead of ~1,000. **79% token savings.**

### What we explored and rejected

| Approach | Why rejected |
|---|---|
| **YOLO / NanoDet object detection** | Outputs generic labels ("person, whiteboard, laptop") — can't describe a chemistry diagram or math equation. Too generic for any real domain. |
| **CLIP embeddings** | 350MB+ model, 1-3s per frame on CPU. Good for similarity matching but doesn't generate descriptions. Overkill for our task. |
| **SmolVLM-256M / Moondream 0.5B** | Full VLMs with instruction following, reasoning, QA — we don't need any of that. They're 500MB-1GB RAM and 5-10s per image on CPU. Too heavy for "just describe what you see." |
| **Mixture of Experts (multiple tiny classifiers)** | Stacking scene classifier + object detector + text detector. Produces structured labels not natural descriptions. Can't handle chemistry, math, or any specialized domain without a specialist per domain. |
| **Face detection as frame filter** | Only works when face fills the frame. This tutorial video had a small face in corner of most frames — detected <15% face area. Useless for filtering. |
| **Audio event detection (music/SFX/lyrics)** | Background music + speech overlap too common. Spectral features can't cleanly separate them. Would falsely label speech as music. Dropped. |
| **Composite labels (NanoDet + SqueezeNet + text detector)** | "classroom \| person, whiteboard \| contains_text" — tells the LLM nothing it can't see in the image itself. Can't describe a complex chemistry diagram. |

### What we learned
The only thing that can describe arbitrary visual content is an encoder-decoder model trained on that content. But it doesn't need to be big — description is pattern matching, not reasoning. A 40M model (LightCap) achieves SOTA captioning. A 60M model focused on structured visual domains is very feasible.

---

## What Iris Is

A graphics describer. It describes visual structure — things where the **shape, layout, and spatial relationships** are the information.

If you gave someone the Iris output instead of the image, they should be able to reconstruct what was shown. Not pixel-perfect — but structurally faithful. The test: can someone draw what was shown from just the description?

## What Iris Is NOT

- **Not OCR.** Plain text on screen, code characters in an editor, slide titles — that's OCR. Tesseract or the API LLM reads text directly. Iris does not waste capacity on character recognition.
- **Not a reasoning model.** It doesn't explain what a diagram means. It describes what it looks like. "Three arrows converging on a central node" — not "this shows a bottleneck in the pipeline."
- **Not a VLM.** No instruction following, no question answering, no chain-of-thought. Input image → output description. That's it. This is why it can be 60M params instead of 256M+.
- **Not a replacement for images in ALL cases.** ~15% of visual content (medical imaging, fine art, complex spatial demonstrations) can't be faithfully described in text. Lens identifies these and sends the actual image.

---

## Every Domain Iris Must Handle

### Mathematics
OCR cannot faithfully capture math. Fractions, integrals, matrices, summations — the spatial arrangement IS the notation. A fraction bar means division. A superscript means exponent. Position encodes meaning.

| Visual | Iris output |
|---|---|
| Fraction with nested integral | `$$\frac{d}{dx}\int_0^x f(t)\,dt = f(x)$$` |
| Matrix | `$$\begin{bmatrix} a & b \\ c & d \end{bmatrix}$$` |
| Summation with bounds | `$$\sum_{i=1}^{n} x_i^2$$` |
| Limit notation | `$$\lim_{x \to 0} \frac{\sin x}{x} = 1$$` |
| Geometric proof figure | "Triangle ABC with right angle at C. Altitude CD to hypotenuse AB. Segments AD=3, DB=12 marked." |
| Graph of function | "Parabola y=x²-4 opening upward, vertex at (0,-4), crossing x-axis at (-2,0) and (2,0), y-axis at (0,-4)." |

**Output format:** LaTeX for equations, plain structured text for geometric figures and graphs.

**Proof this works:** Meta's Nougat achieves >96% text accuracy and ~75% formula accuracy converting PDFs to LaTeX at 250M params. Iris at 60M focused purely on math-to-LaTeX should hit 90%+ on common notation.

### Chemistry
Molecular structures are purely visual. Bond types (single, double, triple, aromatic), ring systems, stereochemistry (wedge/dash bonds), functional groups — none of this is text.

| Visual | Iris output |
|---|---|
| Benzene ring | `SMILES: c1ccccc1` + "Six-membered aromatic ring, all carbons equivalent" |
| Acetic acid structure | `SMILES: CC(=O)O` + "Methyl group bonded to carboxyl group (C=O and O-H)" |
| Reaction mechanism | "Nucleophilic addition: OH⁻ attacks carbonyl carbon from above. Curved arrow from O lone pair to C=O π* orbital. Tetrahedral intermediate formed." |
| Orbital diagram | "sp³ hybridization of carbon: four lobes at 109.5° angles. Two lobes contain bonding pairs (solid), two contain lone pairs (stippled)." |
| Phase diagram | "Three regions: solid (lower-left), liquid (upper-center), gas (lower-right). Triple point at 0.01°C/611 Pa. Critical point at 374°C/22.1 MPa. Positive slope solid-liquid line." |

**Output format:** SMILES notation for molecular structures + plain text for spatial/mechanistic descriptions.

**Proof this works:** Img2Mol has 11.1M molecular structure→SMILES training pairs. MolScribe achieves 76-93% accuracy. DECIMER handles hand-drawn structures. The data and benchmarks exist.

### Biology
Cell diagrams, anatomical cross-sections, phylogenetic trees, protein structures, metabolic pathways — all visual structure.

| Visual | Iris output |
|---|---|
| Animal cell diagram | "Cross-section of animal cell. Nucleus (center, dark) with nucleolus. Rough ER (studded with ribosomes) surrounding nucleus. Mitochondria (oval, double-membrane with cristae) scattered. Golgi apparatus (stacked cisternae) near nucleus. Cell membrane (outer boundary)." |
| Phylogenetic tree | "Rooted tree with 5 terminal taxa. Outgroup A diverges first. B and C form a clade (bootstrap 98%). D and E sister taxa (bootstrap 73%). Time axis left-to-right, scale bar = 10 MYA." |
| Pathway diagram | "Glycolysis pathway: Glucose → (hexokinase, -1 ATP) → G6P → (phosphoglucose isomerase) → F6P → (PFK-1, -1 ATP) → F1,6BP → ... → 2 Pyruvate. Net: 2 ATP + 2 NADH." |

### Physics
Force diagrams, circuits, ray optics, field lines, wave diagrams — vectors and spatial relationships.

| Visual | Iris output |
|---|---|
| Free body diagram | "Block on inclined plane (30°). Forces: Weight mg (downward, 50N), Normal N (perpendicular to surface), Friction f (up the slope), Applied force F (horizontal, 20N)." |
| Circuit schematic | "Series-parallel circuit: Battery 12V → R1 (100Ω) → parallel branch [R2 (200Ω) ∥ R3 (300Ω)] → ground. Ammeter in series before R1. Voltmeter across parallel branch." |
| Ray diagram | "Converging lens, f=10cm. Object at 25cm (left). Image formed at 16.7cm (right), inverted, real, diminished. Principal rays: parallel→through F', through center→straight, through F→parallel." |
| Wave interference | "Two coherent sources S1, S2 separated by 3λ. Constructive interference along central axis and at ±30°, ±90°. Destructive at ±14°, ±48°. Intensity pattern shown as concentric arcs." |

### Engineering & Architecture
System diagrams, circuit boards, mechanical drawings, network topologies, UML, ERDs.

| Visual | Iris output |
|---|---|
| System architecture | "3-tier architecture. Client tier: [Browser, Mobile App] → API Gateway (load balancer) → Service tier: [Auth Service, Payment Service, Inventory Service] (each with own DB) → Message Queue → Analytics Pipeline → Data Warehouse." |
| UML class diagram | "Class Vehicle (abstract): attributes [make: String, model: String, year: int], methods [start(), stop()]. Car extends Vehicle: adds [numDoors: int]. Truck extends Vehicle: adds [payload: double]. Aggregation: Vehicle ◇→ Engine (1:1)." |
| Network topology | "Star topology: Central switch connected to 8 nodes. Nodes 1-4: workstations (10Gbps). Nodes 5-6: servers (25Gbps). Node 7: NAS (10Gbps). Node 8: uplink to router → WAN." |
| Mechanical cross-section | "Bearing assembly: Outer race (steel, press-fit in housing), inner race (press-fit on shaft, Ø25mm), 8 ball bearings in cage, grease-filled cavity, two rubber seals (contact type), snap ring retainer." |

### Data Visualization
Charts where the **shape and pattern** carry meaning beyond the raw numbers.

| Visual | Iris output |
|---|---|
| Bar chart with trend | "Vertical bar chart, 4 bars. Heights increasing left-to-right except 3rd bar dips. Pattern: growth, growth, slight decline, recovery to highest value. Y-axis starts at 0." |
| Scatter plot | "Scatter plot ~200 points. Strong positive correlation (r≈0.85). Two outlier clusters: one at high-x/low-y, one at low-x/high-y. Linear trendline shown, positive slope." |
| Heatmap | "10×10 heatmap. Hot spots (red) in upper-right quadrant and lower-left corner. Cool band (blue) running diagonally from upper-left to lower-right. Color scale: blue (0) to red (100)." |
| Sankey diagram | "Flow from 3 sources to 5 destinations. Source A (largest, 45%) splits to Dest 1 (60%) and Dest 2 (40%). Source B (30%) entirely to Dest 3. Source C (25%) splits across Dest 4 and 5 equally." |

### IDE / Code Context
OCR reads the code characters. Iris describes the **visual state of the development environment** — what panels are open, where errors are, what the developer is looking at.

| Visual | Iris output |
|---|---|
| VS Code with error | "VS Code dark theme. Left sidebar: file explorer open, `src/api/` expanded. Active file: `routes.py` (tab). Line 42 highlighted red — squiggly underline. Problems panel (bottom): 1 error, 2 warnings. Terminal panel: hidden." |
| Diff view | "GitHub diff view: file `auth.rs`. 3 hunks shown. Lines 15-18: removed (red, old implementation). Lines 15-22: added (green, new implementation with error handling). Line 45: single line change. +12 -6 total." |
| Debugger state | "VS Code debugger active. Breakpoint hit at line 78 (yellow arrow). Variables panel: `user_id=42`, `response=None`, `retry_count=3`. Call stack: 5 frames, current frame `process_request()`. Watch: `len(queue)=17`." |

### Slide / Presentation Layout
OCR reads the text. Iris describes **how the slide is structured** — hierarchy, emphasis, positioning, embedded visuals.

| Visual | Iris output |
|---|---|
| Comparison slide | "Two-column layout. Left column header: 'Before' (red). Right column header: 'After' (green). 4 bullet points each side. Center divider line. Bottom: key metrics in bold — '3x improvement'." |
| Process slide | "5-step horizontal process flow. Numbered circles connected by arrows. Step 3 highlighted (larger, different color). Below each step: 2-3 bullet points. Title at top, subtitle in gray." |
| Slide with embedded diagram | "Title slide with embedded flowchart (right 60% of slide). Left side: 3 bullet points with icons. Flowchart has 4 nodes in diamond/rectangle shapes, decision paths labeled Yes/No." |

### Maps & Geography
Routes, regions, spatial relationships, annotations.

| Visual | Iris output |
|---|---|
| Annotated city map | "City map, downtown area. Highlighted route (blue) from point A (northwest) to point B (southeast), 2.3km. Route passes through 3 marked intersections. Red zone: construction area (avoided). Green markers: 4 restaurants along route." |
| Geological cross-section | "Vertical cross-section, 500m depth. Three sedimentary layers: sandstone (top, 50m), limestone (middle, 200m), shale (bottom, 250m). Normal fault at 30° angle displacing layers 40m. Water table at 80m depth." |

### Medical / Anatomical
Annotated imaging, anatomical illustrations, histology.

| Visual | Iris output |
|---|---|
| Annotated X-ray | "Chest X-ray, PA view. Arrow pointing to right lower lobe — 4cm opacity with irregular margins. Heart silhouette normal. Costophrenic angles sharp. No pleural effusion. Trachea midline." |
| Anatomical diagram | "Heart cross-section, 4 chambers labeled. RA (upper-right) → tricuspid valve → RV (lower-right) → pulmonary valve → pulmonary artery. LA (upper-left) ← pulmonary veins. LA → mitral valve → LV (lower-left) → aortic valve → aorta. Septum divides left/right." |

### Musical Notation
Staff, notes, dynamics — structural information.

| Visual | Iris output |
|---|---|
| Sheet music excerpt | `ABC: X:1\nM:4/4\nK:C\nL:1/4\n|: C D E F | G2 G2 | A A A A | G4 :|` + "Two-staff system, treble clef, 4/4 time, key of C major. 4 bars with repeat signs. Ascending scale motif (C-D-E-F) followed by half notes (G-G), quarter notes (A×4), whole note (G)." |

---

## Architecture

```
┌─────────────┐     ┌──────────────┐     ┌─────────────────┐
│  Vision      │     │  Cross-Modal │     │  Text Decoder   │
│  Encoder     │────▶│  Adapter     │────▶│  (autoregressive)│
│              │     │              │     │                 │
│  ViT-Small   │     │  Linear proj │     │  6-layer        │
│  or DINOv2-S │     │  + LayerNorm │     │  transformer    │
│              │     │              │     │                 │
│  22M params  │     │  2-5M params │     │  30-40M params  │
│  FROZEN      │     │  TRAINED     │     │  TRAINED        │
└─────────────┘     └──────────────┘     └─────────────────┘

Total: ~55-65M params
Trainable: ~35-45M params (encoder frozen)
Disk (INT8): ~62MB
RAM at inference: ~150MB (INT8), ~60-80MB (pruned)
Speed: ~55-80ms per image on Apple Silicon
```

### Why freeze the encoder?

ViT-Small / DINOv2-S are pre-trained on millions of images. They already "see" — edges, shapes, spatial relationships, textures. We only train:
- The adapter: how to translate vision features into text space
- The decoder: how to generate structured text descriptions

This saves 22M params from gradient computation and cuts training time in half. Research confirms this works — BLIP-2 freezes both its encoder AND its LLM, training only the 188M Q-Former bridge (2% of total params) and achieves SOTA.

### Output format

The decoder learns to output the right format per content type:

```
[MATH]      → LaTeX notation
[CHEMISTRY] → SMILES + structural description
[DIAGRAM]   → Nested hierarchy with arrows (→) for connections
[CHART]     → Chart type + pattern description + key values
[CIRCUIT]   → Component list with connections
[ANATOMY]   → Labeled structure with spatial relationships
[SLIDE]     → Layout description (columns, hierarchy, positioning)
[IDE]       → Panel state, active file, error locations
[MAP]       → Routes, regions, spatial relationships
[MUSIC]     → ABC notation + structural description
[FIGURE]    → General scientific figure description
```

Format tag is predicted by the model as the first token — it learns which format to use from training data.

---

## Training Data

### Existing datasets — 6M+ images with descriptions

**Tier 1: Primary training sources (use all of these)**

| Dataset | Images | Domain | Description type | Size on disk | Format | Download |
|---|---|---|---|---|---|---|
| **SciCap** | 416K figures | arXiv CS (plots, diagrams, tables) | Real paper captions, avg ~50 words | 18.15 GB | JSON + images (Dropbox) | [GitHub](https://github.com/tingyaohsu/SciCap) |
| **SciGraphQA** | 295K graphs | CS/ML scientific graphs | Multi-turn QA pairs (avg 2.23 turns), GPT-4 rated 8.7/10 | 771 MB JSON (images separate) | JSON + image URLs | [HuggingFace](https://huggingface.co/datasets/alexshengzhili/SciGraphQA-295K-train) |
| **SciMMIR** | 530K figures + tables | Scientific papers (all fields) | Curated captions with 5 subcategories | Not specified | HuggingFace dataset | [GitHub](https://github.com/m-a-p/SciMMIR) + [HuggingFace](https://huggingface.co/datasets/m-a-p/SciMMIR) |
| **Img2Mol** | 11.1M molecules | Chemistry (molecular structures) | SMILES strings (machine-readable chemical notation) | Large (ChEMBL + PubChem) | SMILES + PNG/SVG | [Paper](https://pubs.rsc.org/en/content/articlehtml/2021/sc/d1sc01839f) via ChEMBL/PubChem |
| **DeepPatent2** | 2.8M drawings | Engineering/patent (mechanical, design) | Object-level captions extracted from figure labels, 132K categories | 314 GB | JSON + segmented images | [OneDrive](https://osf.io/fxws7/) |
| **PlotQA** | 224K plots | Real-world data (World Bank, govt) | 28.9M QA pairs from 74 templates | Not specified | JSON + images | [GitHub](https://github.com/NiteshMethani/PlotQA) + [HuggingFace](https://huggingface.co/datasets/achang/plot_qa) |

**Tier 2: Supplementary sources (use for specific domains)**

| Dataset | Images | Domain | Description type | Download |
|---|---|---|---|---|
| **AI2D** | 4.9K diagrams | Science textbook (grades 1-6) | Dense parse graphs (nodes + relationships) + GPT-4V captions via AI2D-Caption | [Allen AI](https://prior.allenai.org/projects/diagram-understanding) (AWS Open Data) |
| **AI2D-RST** | 1K diagrams | Science textbook | Rhetorical Structure Theory annotations (hierarchical discourse) | [GitHub](https://github.com/thiippal/AI2D-RST) |
| **AI2D-Caption** | 4.9K diagrams | Science textbook | LLM-generated captions (GPT-4V + LLaVA 1.5) | [HuggingFace](https://huggingface.co/datasets/abhayzala/AI2D-Caption) |
| **ChartQA** | 18.3K charts | Bar, line, pie charts | 32.7K questions (9.6K human + 23.1K synthetic), includes chart annotations | [GitHub](https://github.com/vis-nlp/ChartQA) |
| **DVQA** | 300K+ bar charts | Synthetic bar charts | 3M QA pairs testing structure/data/reasoning | [GitHub](https://github.com/kushalkafle/DVQA_dataset) — 6.5 GB images + 750 MB QA |
| **FigureQA** | 100K+ charts | Synthetic (line, bar, pie, dot-line) | 1M+ QA pairs + bounding boxes + numerical data | [Microsoft Research](https://www.microsoft.com/en-us/research/project/figureqa-dataset/) — 6+ GB |
| **MultiCaRe** | 135.6K images | Medical/clinical (oncology, cardiology, surgery) | Full captions + labels from 96K cases | [Zenodo](https://zenodo.org/records/10079370) + [HuggingFace](https://huggingface.co/datasets/openmed-community/multicare-case-images) |
| **DECIMER** | 5K molecules | Hand-drawn chemical structures | SMILES strings + SD files | [GitHub](https://github.com/Kohulan/DECIMER-Image-Segmentation) |
| **TQA** | 26K questions | Science textbook (grades 6-8) | Diagram + text QA, 1,076 lessons | [Allen AI](https://allenai.org/data/tqa) — 1.6 GB |
| **IconQA** | 107K QA pairs | Abstract diagrams, logic puzzles | Structured QA + Icon645 dataset (645K icons, 377 classes) | [IconQA](https://iconqa.github.io/) (Google Drive + S3) |
| **Diagram Caption** | 831 diagrams | General diagrams (not photos) | 5.73 human-written captions per image (2-9 range) | [GitHub](https://github.com/yuri-ocha/DiagramCaption) |
| **DocFigure** | 33K figures | CS papers (CVPR/ECCV/ICCV) | 28 figure type class labels (classification only, no descriptions) | [IIIT](https://cvit.iiit.ac.in/usodi/Docfig.php) |
| **SciFIBench** | 2K questions | CS arXiv figures | Figure↔caption matching (human-verified) | [GitHub](https://github.com/jonathan-roberts1/SciFIBench) + [HuggingFace](https://huggingface.co/datasets/jonathan-roberts1/SciFIBench) |
| **ACL-Fig** | 112K figures | NLP/computational linguistics | Full captions + inline references. Pilot: 1,671 manually labeled (19 categories). License: CC BY-NC | [HuggingFace](https://huggingface.co/datasets/citeseerx/ACL-fig) |

### Gaps — need to generate (~20-25K images)

These domains aren't well-covered by existing datasets. Collect screenshots and label with **Gemma batch on local GPU** (user has RTX 5090 with Gemma available).

| Domain | Count needed | Where to collect source images | Labeling approach |
|---|---|---|---|
| **IDE/editor context** | ~5-10K | Script puppeteer/playwright to screenshot VS Code, IntelliJ, Vim, terminal in various states (errors, debugger, diff view, multiple panels) | Gemma batch: "Describe the IDE state — panels, tabs, errors, highlighted lines. Don't read the code, describe the layout." |
| **Slide layouts** | ~5-10K | Google Slides templates, SlideShare (CC), conference talk recordings (screenshot slides), PowerPoint exports | Gemma batch: "Describe the slide layout — columns, hierarchy, positioning, embedded visuals. Don't read the text, describe the structure." |
| **UI wireframes** | ~3-5K | Dribbble (CC), Figma community files, Balsamiq examples, Material Design showcase | Gemma batch: "Describe the UI layout — navigation, content areas, sidebar, modals, form elements, button placement." |
| **System architecture diagrams** | ~3-5K | Tech blog posts (AWS, GCP architecture diagrams), documentation sites, O'Reilly books | Gemma batch: "Describe the architecture — components, connections, data flow direction, layers, services." |
| **Musical notation** | ~2-3K | IMSLP (public domain scores), MuseScore community, LilyPond rendered examples | Gemma batch: "Describe the notation — clef, time signature, key, notes, rests, dynamics, articulations." |
| **Math (supplement)** | ~5K | im2latex dataset (formula images → LaTeX), arXiv rendered equations | Gemma batch: "Output the LaTeX for this equation exactly." |

### Data preparation pipeline (step by step)

```
Step 1: Download existing datasets
────────────────────────────────────
# SciCap (18 GB)
git clone https://github.com/tingyaohsu/SciCap
# Follow Dropbox links in README for images

# SciGraphQA (771 MB)
huggingface-cli download alexshengzhili/SciGraphQA-295K-train

# SciMMIR (530K pairs)
huggingface-cli download m-a-p/SciMMIR

# AI2D + AI2D-Caption
# Download from Allen AI AWS Open Data Registry
huggingface-cli download abhayzala/AI2D-Caption

# Img2Mol — download pre-rendered molecular images from PubChem
# or generate from SMILES using RDKit:
# from rdkit.Chem import Draw, MolFromSmiles
# img = Draw.MolToImage(MolFromSmiles("CC(=O)O"))

# PlotQA
git clone https://github.com/NiteshMethani/PlotQA

# ChartQA
git clone https://github.com/vis-nlp/ChartQA

# MultiCaRe
# Download from Zenodo DOI 10.5281/zenodo.10079370

# DeepPatent2 (314 GB — download selectively if storage limited)
# Available via OneDrive: https://osf.io/fxws7/


Step 2: Reformat captions to Iris output schema (Gemma batch)
──────────────────────────────────────────────────────────────
For each dataset, run Gemma in batch mode to convert existing
captions into our structured format with format tags.

Prompt template for Gemma:
"""
You are reformatting a figure caption into a structured description.
The original caption is from a scientific paper.

Original caption: {caption}
Figure type: {type if available}

Reformat as:
1. Start with the appropriate tag: [MATH], [CHEMISTRY], [DIAGRAM],
   [CHART], [CIRCUIT], [ANATOMY], [FIGURE]
2. Describe the VISUAL STRUCTURE — shapes, connections, spatial layout
3. For math: output LaTeX
4. For chemistry: output SMILES + structural description
5. For charts: describe type, trend, pattern — not just data values
6. Keep under 50 tokens
"""

# Process in batches of 1000 on RTX 5090 using Gemma
# Estimated throughput: ~500-1000 reformats/minute with Gemma 2B
# Total: ~500K reformats × ~1 min/1000 = ~8 hours


Step 3: Generate gap data (IDE, slides, UI, architecture, music)
─────────────────────────────────────────────────────────────────
# Collect screenshots (scripted)
python3 collect_ide_screenshots.py      # puppeteer/playwright
python3 collect_slide_screenshots.py    # Google Slides API
python3 collect_ui_screenshots.py       # Dribbble/Figma scraper

# Label with Gemma batch
python3 label_with_gemma.py --input gap_images/ --output gap_labels.json

# Expected: ~20-25K labeled pairs in ~4-8 hours


Step 4: Filter and validate
────────────────────────────
- Drop pairs where description < 10 tokens or > 200 tokens
- Validate format tags: ensure [MATH] outputs contain $$, [CHEMISTRY] contains SMILES
- Check for hallucination: flag descriptions that mention objects/concepts 
  not plausibly visible in the image (manual review of 500 random samples)
- Remove exact-duplicate images (pHash, Hamming ≤ 4)
- Expected: ~10-15% dropped → ~450K-900K surviving pairs


Step 5: Create train/val/test splits
──────────────────────────────────────
- 80% train / 10% validation / 10% test
- Stratified by domain (ensure each domain has proportional representation)
- No image leakage across splits (same source paper's figures all in same split)


Step 6: Data augmentation
──────────────────────────
- Random crop (90-100% of image area)
- Color jitter (brightness ±10%, contrast ±10%)
- JPEG compression artifacts (quality 60-95, randomly sampled)
- Resolution variation (resize to 160-320px, then to 224×224 for input)
- Horizontal flip (EXCEPT for: math equations, text-heavy, musical notation, chemistry with stereochemistry)
- Effective multiplier: ~2-3x
```

### Total training data

| Source | Pairs | After filtering |
|---|---|---|
| SciCap (reformatted) | 416K | ~350K |
| SciGraphQA (reformatted) | 295K | ~250K |
| SciMMIR (reformatted) | 530K | ~400K (overlap with SciCap removed) |
| Img2Mol (subset) | 100K (of 11.1M) | ~90K |
| PlotQA (reformatted from QA) | 50K (of 224K) | ~40K |
| AI2D + AI2D-Caption | 4.9K | ~4K |
| ChartQA + DVQA + FigureQA | 50K | ~40K |
| MultiCaRe (medical) | 50K (of 135K) | ~40K |
| DeepPatent2 (subset) | 50K (of 2.8M) | ~40K |
| Gap data (IDE, slides, UI, arch, music) | 25K | ~22K |
| **Total before augmentation** | | **~1.28M pairs** |
| **After 2-3x augmentation** | | **~2.5-3.8M effective examples** |

---

## Training

### Hardware: RTX 5090 (32GB VRAM)

```
Optimizer:        AdamW (lr=3e-4, weight_decay=0.01)
Scheduler:        Cosine annealing, 500 steps warmup
Batch size:       64 (frozen encoder = low memory)
Precision:        FP16 (PyTorch AMP)
Epochs:           20-30 with early stopping
Gradient clip:    max_norm=1.0
Loss:             Cross-entropy on decoder output (causal LM loss)
```

### Training time

| Phase | Time |
|---|---|
| Download + reformat existing datasets | 1-2 days |
| Generate gap data (Gemma batch) | 4-8 hours |
| Filter + prepare splits | 2-4 hours |
| **Model training (30 epochs on ~1M pairs)** | **6-12 hours on RTX 5090** |
| Evaluation + iteration | 1 day |
| ONNX export + INT8 quantization | 1-2 hours |
| **Total** | **~4-6 days** |

### Model implementation (PyTorch)

```python
import torch
import torch.nn as nn
from transformers import ViTModel, AutoTokenizer

class ArvelIris(nn.Module):
    """
    Visual structure description model.
    Frozen ViT-Small encoder + trainable cross-modal adapter + trainable decoder.
    """
    def __init__(self, vocab_size=32128, d_model=512, n_heads=8, n_layers=6, max_len=50):
        super().__init__()
        
        # === FROZEN vision encoder (22M params) ===
        # Option A: DeiT-Small (supervised ImageNet pre-training)
        self.encoder = ViTModel.from_pretrained("facebook/deit-small-patch16-224")
        # Option B: DINOv2-Small (self-supervised, better at structure)
        # self.encoder = ViTModel.from_pretrained("facebook/dinov2-small")
        
        for param in self.encoder.parameters():
            param.requires_grad = False
        
        self.encoder_dim = 384  # ViT-Small hidden size
        
        # === TRAINABLE cross-modal adapter (3-5M params) ===
        self.adapter = nn.Sequential(
            nn.Linear(self.encoder_dim, d_model),
            nn.LayerNorm(d_model),
            nn.GELU(),
            nn.Linear(d_model, d_model),
            nn.LayerNorm(d_model),
        )
        
        # Token compression: merge 196 patches → 50 learned queries
        self.num_queries = 50
        self.query_tokens = nn.Parameter(torch.randn(1, self.num_queries, d_model) * 0.02)
        self.cross_attn_compress = nn.MultiheadAttention(d_model, n_heads, batch_first=True)
        
        # === TRAINABLE text decoder (30-40M params) ===
        self.token_embedding = nn.Embedding(vocab_size, d_model)
        self.position_embedding = nn.Embedding(max_len, d_model)
        
        decoder_layer = nn.TransformerDecoderLayer(
            d_model=d_model, nhead=n_heads, dim_feedforward=d_model * 4,
            dropout=0.1, batch_first=True, norm_first=True
        )
        self.decoder = nn.TransformerDecoder(decoder_layer, num_layers=n_layers)
        
        self.output_projection = nn.Linear(d_model, vocab_size)
        self.max_len = max_len
        
    def encode_image(self, pixel_values):
        """Encode image to compressed visual tokens. Run once per image."""
        with torch.no_grad():
            vision_output = self.encoder(pixel_values).last_hidden_state  # [B, 197, 384]
            vision_output = vision_output[:, 1:, :]  # drop CLS token → [B, 196, 384]
        
        # Adapt to decoder dimension
        adapted = self.adapter(vision_output)  # [B, 196, 512]
        
        # Compress 196 patches → 50 query tokens via cross-attention
        queries = self.query_tokens.expand(pixel_values.size(0), -1, -1)  # [B, 50, 512]
        compressed, _ = self.cross_attn_compress(queries, adapted, adapted)  # [B, 50, 512]
        
        return compressed
    
    def decode_step(self, token_ids, memory, past_positions=0):
        """Single decoder step (for inference with KV cache)."""
        seq_len = token_ids.size(1)
        positions = torch.arange(past_positions, past_positions + seq_len, device=token_ids.device)
        
        x = self.token_embedding(token_ids) + self.position_embedding(positions)
        
        # Causal mask
        causal_mask = nn.Transformer.generate_square_subsequent_mask(seq_len, device=token_ids.device)
        
        decoded = self.decoder(x, memory, tgt_mask=causal_mask)
        logits = self.output_projection(decoded)
        
        return logits
    
    def forward(self, pixel_values, target_ids):
        """Training forward pass."""
        memory = self.encode_image(pixel_values)  # [B, 50, 512]
        logits = self.decode_step(target_ids[:, :-1], memory)  # teacher forcing
        return logits  # [B, seq_len-1, vocab_size]
    
    @torch.no_grad()
    def generate(self, pixel_values, tokenizer, max_len=50):
        """Greedy decoding for inference."""
        memory = self.encode_image(pixel_values)
        
        # Start with BOS token
        tokens = [tokenizer.bos_token_id]
        
        for _ in range(max_len):
            input_ids = torch.tensor([tokens], device=pixel_values.device)
            logits = self.decode_step(input_ids, memory)
            next_token = logits[0, -1].argmax().item()
            
            if next_token == tokenizer.eos_token_id:
                break
            tokens.append(next_token)
        
        return tokenizer.decode(tokens[1:])  # skip BOS
```

### Training loop

```python
from torch.utils.data import DataLoader
from transformers import get_cosine_schedule_with_warmup

# Model + optimizer
model = ArvelIris().cuda()
optimizer = torch.optim.AdamW(
    [p for p in model.parameters() if p.requires_grad],
    lr=3e-4, weight_decay=0.01
)
scheduler = get_cosine_schedule_with_warmup(optimizer, 500, total_steps)
scaler = torch.amp.GradScaler()  # FP16 mixed precision

# Training
for epoch in range(30):
    for batch in dataloader:
        images = batch["pixel_values"].cuda()       # [B, 3, 224, 224]
        target_ids = batch["input_ids"].cuda()       # [B, max_len]
        
        with torch.amp.autocast(device_type="cuda"):
            logits = model(images, target_ids)       # [B, seq_len-1, vocab]
            loss = F.cross_entropy(
                logits.reshape(-1, logits.size(-1)),
                target_ids[:, 1:].reshape(-1),       # shift right for teacher forcing
                ignore_index=tokenizer.pad_token_id
            )
        
        scaler.scale(loss).backward()
        scaler.unscale_(optimizer)
        torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
        scaler.step(optimizer)
        scaler.update()
        scheduler.step()
        optimizer.zero_grad()
    
    # Validation
    val_loss = evaluate(model, val_dataloader)
    if val_loss < best_val_loss:
        torch.save(model.state_dict(), "iris_best.pt")
        best_val_loss = val_loss
    elif patience_counter > 5:
        break  # early stopping
```

### ONNX export (step by step)

```python
import torch
import onnxruntime as ort
from onnxruntime.quantization import quantize_static, CalibrationDataReader

# === Step 1: Export encoder (frozen ViT + adapter + compression) ===
class IrisEncoder(nn.Module):
    def __init__(self, iris_model):
        super().__init__()
        self.encoder = iris_model.encoder
        self.adapter = iris_model.adapter
        self.query_tokens = iris_model.query_tokens
        self.cross_attn_compress = iris_model.cross_attn_compress
    
    def forward(self, pixel_values):
        with torch.no_grad():
            v = self.encoder(pixel_values).last_hidden_state[:, 1:, :]
        adapted = self.adapter(v)
        queries = self.query_tokens.expand(pixel_values.size(0), -1, -1)
        compressed, _ = self.cross_attn_compress(queries, adapted, adapted)
        return compressed

encoder_module = IrisEncoder(model).eval()
dummy_image = torch.randn(1, 3, 224, 224)

torch.onnx.export(
    encoder_module, dummy_image, "iris_encoder.onnx",
    input_names=["pixel_values"],
    output_names=["visual_tokens"],
    dynamic_axes=None,  # static shape for max optimization
    opset_version=17
)

# === Step 2: Export decoder (one step, for autoregressive) ===
class IrisDecoderStep(nn.Module):
    def __init__(self, iris_model):
        super().__init__()
        self.token_embedding = iris_model.token_embedding
        self.position_embedding = iris_model.position_embedding
        self.decoder = iris_model.decoder
        self.output_projection = iris_model.output_projection
    
    def forward(self, input_ids, visual_tokens, position_offset):
        x = self.token_embedding(input_ids) + self.position_embedding(position_offset)
        decoded = self.decoder(x, visual_tokens)
        logits = self.output_projection(decoded)
        return logits

decoder_module = IrisDecoderStep(model).eval()
dummy_ids = torch.tensor([[0]])
dummy_visual = torch.randn(1, 50, 512)
dummy_pos = torch.tensor([0])

torch.onnx.export(
    decoder_module, (dummy_ids, dummy_visual, dummy_pos),
    "iris_decoder.onnx",
    input_names=["input_ids", "visual_tokens", "position"],
    output_names=["logits"],
    dynamic_axes={"input_ids": {1: "seq_len"}},
    opset_version=17
)

# === Step 3: INT8 quantization ===
class CalibReader(CalibrationDataReader):
    def __init__(self, dataset, limit=100):
        self.data = [dataset[i] for i in range(min(limit, len(dataset)))]
        self.idx = 0
    def get_next(self):
        if self.idx >= len(self.data):
            return None
        item = {"pixel_values": self.data[self.idx]["pixel_values"].numpy()}
        self.idx += 1
        return item

quantize_static("iris_encoder.onnx", "iris_encoder_int8.onnx", CalibReader(val_dataset))
quantize_static("iris_decoder.onnx", "iris_decoder_int8.onnx", CalibReader(val_dataset))

# Final files:
# iris_encoder_int8.onnx  (~22MB — frozen ViT weights quantized)
# iris_decoder_int8.onnx  (~40MB — adapter + decoder quantized)
# Total: ~62MB
```

### Inference code (production, ONNX Runtime)

```python
import onnxruntime as ort
import numpy as np
from PIL import Image
from torchvision import transforms

# Load models (one-time)
encoder = ort.InferenceSession("iris_encoder_int8.onnx", providers=["CPUExecutionProvider"])
decoder = ort.InferenceSession("iris_decoder_int8.onnx", providers=["CPUExecutionProvider"])

# Image preprocessing (must match training)
preprocess = transforms.Compose([
    transforms.Resize((224, 224)),
    transforms.ToTensor(),
    transforms.Normalize(mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]),
])

def describe_frame(image_path: str) -> str:
    """Describe a graphic/diagram/figure in structured text."""
    
    # 1. Preprocess image
    image = Image.open(image_path).convert("RGB")
    pixel_values = preprocess(image).unsqueeze(0).numpy()  # [1, 3, 224, 224]
    
    # 2. Encode (once, ~10-15ms)
    visual_tokens = encoder.run(None, {"pixel_values": pixel_values})[0]  # [1, 50, 512]
    
    # 3. Decode autoregressively with greedy search (~40-60ms)
    tokens = [BOS_TOKEN_ID]
    
    for step in range(50):  # max 50 tokens
        input_ids = np.array([[tokens[-1]]], dtype=np.int64)
        position = np.array([step], dtype=np.int64)
        
        logits = decoder.run(None, {
            "input_ids": input_ids,
            "visual_tokens": visual_tokens,
            "position": position,
        })[0]
        
        next_token = int(np.argmax(logits[0, -1]))
        
        if next_token == EOS_TOKEN_ID:
            break
        tokens.append(next_token)
    
    return tokenizer.decode(tokens[1:])
```

### Evaluation metrics

- **BLEU-4**: n-gram precision against reference descriptions
- **CIDEr**: TF-IDF weighted, consensus-based (standard for captioning)
- **Format compliance**: % of outputs with valid LaTeX/SMILES/structured text
- **Domain accuracy**: Per-domain manual review of 100 samples each
- **Reconstruction test**: Can a human draw/reconstruct the visual from just the description?
- **LaTeX round-trip**: Render Iris LaTeX output → compare rendered image to original (pixel similarity)
- **SMILES validity**: % of SMILES outputs that parse into valid molecules (RDKit validation)

---

## Inference Optimization

### Target: <100ms per image on Apple Silicon

```
Image preprocessing (resize + normalize):    ~2ms
Vision encoder (INT8, static 224×224):      ~10-15ms
Token compression (196 → 50 patches):        ~1ms
Adapter projection:                           ~1ms
Decoder (INT8, KV cache, ~25 tokens):       ~40-60ms  (2-3ms per token)
───────────────────────────────────────────────────────
Total:                                       ~55-80ms
```

### Optimization stack

1. **ONNX Runtime** with INT8 static quantization (2-3x speedup, <1% accuracy loss)
2. **KV cache** for decoder (reuse key/value tensors, ~50x faster generation)
3. **Encoder caching** (process image once, decode many tokens)
4. **Token compression** (merge similar patch embeddings: 196 → 50, 4x less cross-attention)
5. **Greedy decoding** (no beam search — single-pass argmax, ~3-5x faster for ~2% quality tradeoff)
6. **Static shapes** (fixed 224×224 input, max 50 output tokens — enables ONNX graph optimization)

### Final model specs

| Spec | Value |
|---|---|
| Params | ~60M total, ~40M trainable |
| Disk (INT8) | ~62MB |
| Disk (INT8 + 80% pruned) | ~13MB |
| RAM at inference | ~150MB (INT8) |
| RAM (pruned) | ~60-80MB |
| Speed (Apple Silicon) | ~55-80ms per image |
| Speed (any x86 CPU) | ~100-200ms per image |
| Max output | 50 tokens (~30-50 words) |

---

## Expected Accuracy by Domain

| Domain | Target accuracy | Basis |
|---|---|---|
| Math → LaTeX | 90-95% | Nougat does 75% at 250M. Focused math-only training at 60M should exceed with abundant im2latex data |
| Chemistry → SMILES | 85-93% | Img2Mol/MolScribe benchmarks on 11.1M training pairs |
| Simple diagrams (flowcharts) | 85-90% | Boxes + arrows + labels are structured and repetitive |
| Charts (shape + trend) | 85-90% | DePlot/ChartQA prove this. Shape description is tractable |
| Tables (structure) | 80-90% | Grid detection well-studied |
| Biology diagrams | 75-85% | Needs domain vocab but structures are learnable from SciCap/AI2D |
| Physics diagrams | 75-85% | Vectors and circuits are structured — AI2D covers this |
| Engineering schematics | 70-80% | Complex spatial relationships, 2.8M patent drawings help |
| IDE context | 80-90% | Layout is structured (panels, tabs) — gap data needed |
| Slide layout | 85-90% | Hierarchy and positioning — gap data needed |
| Medical imaging | 50-65% | Hardest — least structured, most domain-specific |
| Musical notation | 70-80% | ABC notation is well-defined, limited gap data |

**Where accuracy falls short → Lens sends the actual image as fallback.** The two-pass pipeline already identifies high-complexity frames. Iris describes what it can, the API LLM sees the rest.

---

## How Lens + Iris Work Together

```
Video
  │
  ├─ Arvel Lens: select 21 unique frames from 57
  │
  ├─ For each frame:
  │   ├─ Run Iris (~60ms): get structured description
  │   ├─ Check Iris confidence (average token probability)
  │   │
  │   ├─ HIGH confidence → send TEXT description only (~20 tokens)
  │   └─ LOW confidence → send ACTUAL IMAGE (~1000 tokens)
  │
  ├─ Transcript from Parakeet (all spoken content)
  │
  └─ API LLM receives:
       - Full transcript
       - ~16 text descriptions (320 tokens)
       - ~5 actual images (5,000 tokens)
       - Total: ~5,320 tokens
       
       vs ALL images: 21 × 1,000 = 21,000 tokens
       
       SAVINGS: 75-79%
```

---

## End-to-End Pipeline: Video → API LLM

This is the complete system — Lens selects frames, Iris describes them, OCR reads text, Parakeet transcribes speech.

```
Video file (any length, any content)
  │
  ├─ AUDIO PATH
  │   └─ Parakeet ASR (already built in MacTranscribe)
  │       → Full timestamped transcript of all spoken content
  │
  ├─ VISUAL PATH
  │   ├─ Arvel Lens: frame selection (built, benchmarked)
  │   │   ├─ ffmpeg scene detection → ~57 scene-change frames
  │   │   ├─ Pass 1: edge density gate (≥5%) → protect structured frames
  │   │   ├─ Pass 1b: pHash dedup → screencast support
  │   │   ├─ Pass 2: leader algorithm → deduplicate talking heads
  │   │   └─ Result: ~21 unique frames
  │   │
  │   ├─ For each of the 21 frames:
  │   │   │
  │   │   ├─ Is it text-heavy? (edge density + low contour complexity = text slide)
  │   │   │   └─ YES → OCR only (tesseract or API LLM reads text directly)
  │   │   │         Send: "[SLIDE] OCR text: {extracted text}" (~30 tokens)
  │   │   │
  │   │   ├─ Is it a graphic/diagram/figure? (high edge + high contour complexity)
  │   │   │   └─ YES → Arvel Iris (~60ms)
  │   │   │         ├─ HIGH confidence → send TEXT description (~20-30 tokens)
  │   │   │         └─ LOW confidence → send ACTUAL IMAGE (~1000 tokens)
  │   │   │
  │   │   └─ Is it a natural scene / talking head?
  │   │       └─ YES → brief text description (~10 tokens)
  │   │             "Person at desk with microphone, dark room"
  │   │
  │   └─ Result: mix of text descriptions + few actual images
  │
  └─ API LLM receives:
       ┌──────────────────────────────────────────┐
       │ Transcript: [full timestamped speech]     │
       │                                           │
       │ [0:15] FRAME: [MATH] $$\int_0^1 x^2 dx$$ │
       │ [2:50] FRAME: [DIAGRAM] Flowchart with    │
       │        4 nodes: Start → Process →          │
       │        Decision → End                      │
       │ [4:42] FRAME: [IMAGE] (actual image sent) │
       │ [8:06] FRAME: [DIAGRAM] Roadmap with 4    │
       │        sections: Setup, Basics, Advanced,  │
       │        Projects...                         │
       │ [9:52] FRAME: [IMAGE] (actual image sent) │
       └──────────────────────────────────────────┘
       
       Token budget:
       - Transcript: ~15,000 tokens
       - ~16 text descriptions: ~400 tokens
       - ~5 actual images: ~5,000 tokens
       - TOTAL: ~20,400 tokens
       
       vs ALL frames as images: 21 × 1,000 = 21,000 + 15,000 = 36,000 tokens
       
       SAVINGS: ~43% on this example (more on graphics-heavy content)
```

### Confidence thresholding

During Iris inference, track the average log-probability of generated tokens:

```python
def describe_with_confidence(image_path):
    description, token_probs = iris_generate_with_probs(image_path)
    avg_confidence = sum(token_probs) / len(token_probs)
    
    if avg_confidence > CONFIDENCE_THRESHOLD:  # e.g., -0.5
        return {"type": "text", "content": description}
    else:
        return {"type": "image", "path": image_path}
```

Low confidence means Iris is uncertain — the graphic is too complex or unfamiliar. Send the actual image instead of a bad description.

---

## References

### Models that prove this works
- **Nougat** (Meta, 2023): PDF → Markdown with LaTeX. >96% text, ~75% formula accuracy. Proves structured output from visual input is tractable. [arXiv:2308.13418](https://arxiv.org/abs/2308.13418)
- **LightCap** (AAAI, 2023): 40M param captioner, 136.6 CIDEr on COCO. Proves tiny models can caption at SOTA level. [AAAI 2023](https://ojs.aaai.org/index.php/AAAI/article/view/25359)
- **DePlot** (Google, 2023): Chart → data table, +24-29% over prior SOTA. Proves chart derendering works. [arXiv:2212.10505](https://arxiv.org/abs/2212.10505)
- **Pix2Struct** (Google, 2023): Screenshot → structured HTML, SOTA 6/9 benchmarks. Proves image-to-structured-text works. [arXiv:2210.03347](https://arxiv.org/abs/2210.03347)
- **MolScribe** (2023): Molecular image → SMILES, 76-93% accuracy. Proves chemical structure recognition. [J. Chem. Inf. Model. 2023](https://pubs.acs.org/doi/10.1021/acs.jcim.2c01480)
- **GIT** (Microsoft, 2022): Surpassed human performance on TextCaps (138.2 vs 125.5 CIDEr). [arXiv:2205.14100](https://arxiv.org/abs/2205.14100)

### Training data methodology
- **ShareGPT4V**: 100K GPT-4V captions → 1.2M via trained model. Avg 942 chars. Proves synthetic caption generation works at scale. [arXiv:2311.12793](https://arxiv.org/abs/2311.12793)
- **ALLaVA**: 4B models trained on high-quality synthetic data match 7B/13B performance. Quality > quantity. [arXiv:2402.11684](https://arxiv.org/abs/2402.11684)
- **BLIP**: Bootstrapped captioning — generate synthetic captions, filter with quality model, iterate. +2.8% CIDEr. [arXiv:2201.12086](https://arxiv.org/abs/2201.12086)

### Architecture decisions
- **Frozen encoder**: BLIP-2 freezes both encoder + LLM, trains only Q-Former bridge (2% of params). [arXiv:2301.12597](https://arxiv.org/abs/2301.12597)
- **Knowledge distillation**: Student models achieve 85-95% of teacher accuracy at 10-50x fewer params. [arXiv:2501.13341](https://arxiv.org/abs/2501.13341)
- **LoRA fine-tuning**: <1% parameter overhead, achieves full fine-tuning performance. [arXiv:2106.09685](https://arxiv.org/abs/2106.09685)

### CPU inference optimization
- **ONNX Runtime INT8**: 2-3x CPU speedup, <1% accuracy loss with calibration. [ONNX Runtime docs](https://onnxruntime.ai/docs/performance/model-optimizations/quantization.html)
- **KV cache**: ~50x faster autoregressive generation. [ONNX GenAI docs](https://onnxruntime.ai/docs/genai/)
- **BERT-base on M2 Max**: 38.23ms — baseline for transformer speed on Apple Silicon.
- **LightCap**: 188ms on smartphone CPU at 40M params — our 60M target is realistic.

### Datasets
- SciCap: [arXiv:2110.11624](https://arxiv.org/abs/2110.11624) — 416K scientific figures with captions
- SciGraphQA: [arXiv:2308.03349](https://arxiv.org/abs/2308.03349) — 295K graphs with multi-turn QA
- AI2D: [Allen AI](https://prior.allenai.org/projects/diagram-understanding) — 4.9K science diagrams with structural annotations
- Img2Mol: [RSC 2021](https://pubs.rsc.org/en/content/articlehtml/2021/sc/d1sc01839f) — 11.1M molecular depictions → SMILES
- DeepPatent2: [Nature 2023](https://www.nature.com/articles/s41597-023-02653-7) — 2.8M patent drawings
- MultiCaRe: [Zenodo](https://zenodo.org/records/10079370) — 135K clinical images with captions
- SciMMIR: [arXiv:2401.13478](https://arxiv.org/abs/2401.13478) — 530K scientific figure-caption pairs
