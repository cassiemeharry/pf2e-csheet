use pragmatic_segmenter::Segmenter;
// use pretty::RcDoc;
// use smartstring::alias::String;

// pub use self::perceptron::PerceptronTagger;

lazy_static::lazy_static! {
    // pub static ref TAGGER: PerceptronTagger =
    //     PerceptronTagger::new_trained()
    //     .expect("Failed to load tagger. Maybe you need to train it first?");

    pub static ref SEGMENTER: Segmenter = Segmenter::new().unwrap();
}

// #[derive(Clone, Debug)]
// pub struct TagResult<'a> {
//     pub word: &'a str,
//     // pub pos_tag: String,
//     pub chunk_tag: String,
// }

// #[derive(Clone, Debug)]
// pub enum SentenceTree {
//     Leaf {
//         word: String,
//         // pos_tag: &'a str,
//     },
//     Subtree {
//         label: String,
//         children: Vec<SentenceTree>,
//     },
// }

// impl SentenceTree {
//     fn push(&mut self, node: Self) {
//         match self {
//             Self::Leaf { .. } => todo!(),
//             Self::Subtree { label: _, children } => children.push(node),
//         }
//     }

//     fn as_subtree_parts_mut(&mut self) -> Option<(&str, &mut Vec<Self>)> {
//         match self {
//             &mut Self::Leaf { .. } => None,
//             &mut Self::Subtree {
//                 ref label,
//                 ref mut children,
//             } => Some((label.as_str(), children)),
//         }
//     }

//     fn to_doc(&self) -> RcDoc<()> {
//         match self {
//             Self::Leaf { word } => RcDoc::text(word.as_str()),
//             Self::Subtree { label, children } => RcDoc::text("(")
//                 .append(RcDoc::text(label.as_str()))
//                 .append(RcDoc::space())
//                 .append(
//                     RcDoc::intersperse(children.iter().map(|t| t.to_doc()), RcDoc::line())
//                         .nest(1)
//                         .group(),
//                 )
//                 .append(RcDoc::text(")")),
//         }
//     }
// }

// impl std::fmt::Display for SentenceTree {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         let width = f.width().unwrap_or(80);
//         self.to_doc().render_fmt(width, f)
//     }
// }

// pub fn sentences_to_trees<'s>(sentences: &'s str) -> impl Iterator<Item = SentenceTree> + 's {
//     SEGMENTER
//         .segment(sentences)
//         .into_iter()
//         .enumerate()
//         .map(|(i, sentence)| {
//             trace!("Converting sentence {} to tree: {:?}", i + 1, sentence);
//             let tagged: Vec<TagResult<'_>> = tag_sentence(sentence);
//             let tree: SentenceTree = conlltags2tree(&tagged);
//             tree
//         })
// }

// fn tag_sentence<'s>(s: &'s str) -> Vec<TagResult<'s>> {
//     let r = regex::Regex::new(r##"\w+|[.,?!;"']"##).unwrap();
//     let words: Vec<&str> = r.find_iter(s).map(|m| m.as_str()).collect();
//     let tagged = TAGGER.tag(&words, false, true);
//     tagged.into_iter().map(|(tagged, _)| tagged).collect()
// }

// fn conlltags2tree<'s>(sentence: &[TagResult<'s>]) -> SentenceTree {
//     debug!("Converting conll tagged sentence into tree");
//     let mut tree = SentenceTree::Subtree {
//         label: "S".into(),
//         children: vec![],
//     };
//     for tagged in sentence {
//         let leaf = SentenceTree::Leaf {
//             word: tagged.word.into(),
//             // pos_tag: tagged.pos_tag.as_str(),
//         };
//         if tagged.chunk_tag.starts_with("B-") {
//             tree.push(SentenceTree::Subtree {
//                 label: tagged.chunk_tag[2..].into(),
//                 children: vec![leaf],
//             });
//         } else if tagged.chunk_tag.starts_with("I-") {
//             match &mut tree {
//                 SentenceTree::Leaf { .. } => todo!(),
//                 SentenceTree::Subtree {
//                     label: tree_label,
//                     children: tree_children,
//                 } => {
//                     match tree_children.last_mut() {
//                         None | Some(SentenceTree::Leaf { .. }) => {
//                             // as B-*
//                             debug!(
//                                 "Tried to append intra-chunk word {:?} to leaf node {:?}",
//                                 tagged,
//                                 tree_children.last(),
//                             );
//                             tree.push(SentenceTree::Subtree {
//                                 label: tagged.chunk_tag[2..].into(),
//                                 children: vec![leaf],
//                             });
//                         }
//                         Some(SentenceTree::Subtree {
//                             label: last_label,
//                             children: last_children,
//                         }) => {
//                             if last_label.as_str() != &tagged.chunk_tag[2..] {
//                                 // as B-*
//                                 debug!(
//                                     "Tried to append intra-chunk word {:?} to mismatched label {:?} (expected {:?})",
//                                     tagged,
//                                     last_label,
//                                     &tagged.chunk_tag[2..],
//                                 );
//                                 tree.push(SentenceTree::Subtree {
//                                     label: tagged.chunk_tag[2..].into(),
//                                     children: vec![leaf],
//                                 });
//                             } else {
//                                 last_children.push(leaf);
//                             }
//                         }
//                     }
//                 }
//             }
//         } else if tagged.chunk_tag.as_str() == "O" {
//             tree.push(leaf);
//         } else {
//             panic!("Bad conll tag {:?}", tagged.chunk_tag);
//         }
//     }
//     tree
// }

// mod conll2000 {
//     use lazy_static::lazy_static;
//     use libflate::gzip;
//     use smallvec::SmallVec;
//     use smartstring::alias::String;
//     use std::{
//         fs::File,
//         io::{BufRead as _, BufReader},
//         path::Path,
//     };

//     lazy_static! {
//         pub(super) static ref TEST_DATA: Conll2000Data = Conll2000Data::from_file(concat!(
//             env!("CARGO_MANIFEST_DIR"),
//             "/conll2000/test.txt.gz"
//         ))
//         .expect("Failed to open conll2000 test data");
//         pub(super) static ref TRAIN_DATA: Conll2000Data = Conll2000Data::from_file(concat!(
//             env!("CARGO_MANIFEST_DIR"),
//             "/conll2000/train.txt.gz"
//         ))
//         .expect("Failed to open conll2000 training data");
//     }

//     #[derive(Clone, Debug)]
//     pub(super) struct WordInfo {
//         pub word: String,
//         pub pos_tag: String,
//         pub chunk_tag: String,
//     }

//     pub(super) struct Conll2000Data {
//         pub sentences: Vec<Vec<WordInfo>>,
//     }

//     impl Conll2000Data {
//         fn from_file(path: impl AsRef<Path>) -> std::io::Result<Self> {
//             debug!("Loading conll2000 data from {}", path.as_ref().display());
//             let f = File::open(path)?;
//             let unzipped = gzip::Decoder::new(f)?;
//             let br = BufReader::new(unzipped);
//             let mut line_count = 0;
//             let mut sentences = vec![];
//             let mut current_sentence = vec![];

//             for line_result in br.lines() {
//                 line_count += 1;
//                 let line = line_result?;
//                 if &line == "" {
//                     if !current_sentence.is_empty() {
//                         sentences.push(current_sentence.clone());
//                         current_sentence.clear();
//                     }
//                     continue;
//                 }
//                 let raw_words = line.split_whitespace().collect::<SmallVec<[&str; 3]>>();
//                 match raw_words.as_slice() {
//                     &[word, pos_tag, chunk_tag] => {
//                         current_sentence.push(WordInfo {
//                             word: word.into(),
//                             pos_tag: pos_tag.into(),
//                             chunk_tag: chunk_tag.into(),
//                         });
//                     }
//                     _ => panic!("Expected exactly 3 words, got {:?}", raw_words),
//                 }
//             }
//             if !current_sentence.is_empty() {
//                 sentences.push(current_sentence);
//             }
//             debug!(
//                 "Loaded {} lines into {} sentences",
//                 line_count,
//                 sentences.len()
//             );
//             Ok(Self { sentences })
//         }
//     }

//     #[test]
//     fn test_loading_data() {
//         let train_sentences = TRAIN_DATA.sentences.len();
//         println!("Found {} train sentences", train_sentences);
//         let test_sentences = TEST_DATA.sentences.len();
//         println!("Found {} test sentences", test_sentences);
//     }
// }

// /// This is heavily based off of the `nltk.tag.perceptron` Python module.
// mod perceptron {
//     use ordered_float::OrderedFloat;
//     use serde::{Deserialize, Serialize};
//     use smartstring::alias::String;
//     use std::collections::{HashMap, HashSet};

//     use super::{conll2000::WordInfo, TagResult};

//     #[derive(Clone, Debug, Deserialize, Serialize)]
//     struct AveragedPerceptron {
//         weights: HashMap<String, HashMap<String, f64>>,
//         classes: HashSet<String>,
//         totals: HashMap<String, HashMap<String, f64>>,
//         timestamps: HashMap<String, HashMap<String, u64>>,
//         instances: u64,
//     }

//     const SAVE_PATH: &'static str =
//         concat!(env!("CARGO_MANIFEST_DIR"), "/conll2000/trained.json.gz");

//     impl AveragedPerceptron {
//         fn new() -> Self {
//             Self::with_weights(HashMap::new())
//         }

//         fn with_weights(weights: HashMap<String, HashMap<String, f64>>) -> Self {
//             Self {
//                 weights,
//                 classes: HashSet::new(),
//                 totals: HashMap::new(),
//                 timestamps: HashMap::new(),
//                 instances: 0,
//             }
//         }

//         fn load() -> anyhow::Result<Self> {
//             let f = std::fs::File::open(SAVE_PATH)?;
//             debug!("Loading serialized tagger from {}...", SAVE_PATH);
//             let decoder = libflate::gzip::Decoder::new(f)?;
//             let buffer = std::io::BufReader::with_capacity(1024 * 4, decoder);
//             let deserialized = serde_json::from_reader(buffer)?;
//             debug!("Finished loading serialized tagger");
//             Ok(deserialized)
//         }

//         fn save(&self) -> anyhow::Result<()> {
//             use std::io::Write;

//             let tmp_path = {
//                 let mut buffer: String = SAVE_PATH.into();
//                 buffer.push_str(".new");
//                 buffer
//             };
//             let f = std::fs::File::create(&*tmp_path)?;
//             let buffer = std::io::BufWriter::with_capacity(1024 * 4, f);
//             let mut encoder = libflate::gzip::Encoder::new(buffer)?;
//             serde_json::to_writer(&mut encoder, self)?;
//             let mut buffer = encoder.finish().into_result()?;
//             buffer.flush()?;
//             std::fs::rename(&*tmp_path, SAVE_PATH)?;
//             Ok(())
//         }

//         fn softmax(scores: impl IntoIterator<Item = f64>) -> impl IntoIterator<Item = f64> {
//             let scores = scores.into_iter();
//             let mut exps: Vec<f64> = Vec::with_capacity(scores.size_hint().0);
//             let mut total = 0.0;
//             for value in scores {
//                 exps.push(value.exp());
//                 total = total + value;
//             }
//             for exp in exps.iter_mut() {
//                 *exp = *exp / total;
//             }
//             exps
//         }

//         /// Dot product the features and the current weights and return the best label.
//         fn predict(
//             &self,
//             features: &HashMap<String, f64>,
//             return_conf: bool,
//         ) -> (&str, Option<f64>) {
//             let mut scores: HashMap<String, f64> = HashMap::new();
//             for (feat, value) in features.iter() {
//                 if *value == 0.0 {
//                     continue;
//                 }
//                 let weights = match self.weights.get(feat) {
//                     Some(ws) => ws,
//                     None => continue,
//                 };
//                 for (label, weight) in weights.iter() {
//                     let prev_score = scores.get(label.as_str()).copied().unwrap_or_default();
//                     let weighted = (*value) * (*weight);
//                     scores.insert(label.clone(), prev_score + weighted);
//                 }
//             }

//             let best_label = self
//                 .classes
//                 .iter()
//                 .max_by_key(|label| {
//                     (
//                         scores
//                             .get(label.as_str())
//                             .copied()
//                             .map(OrderedFloat::from)
//                             .unwrap_or_default(),
//                         label.clone(),
//                     )
//                 })
//                 .expect("No classes were loaded");
//             let conf_opt = if return_conf {
//                 let conf = Self::softmax(scores.values().copied())
//                     .into_iter()
//                     .map(OrderedFloat::from)
//                     .max()
//                     .unwrap_or_default()
//                     .into_inner();
//                 Some(conf)
//             } else {
//                 None
//             };
//             (best_label, conf_opt)
//         }

//         /// Update the feature weights.
//         fn update<'a>(
//             &mut self,
//             truth: &str,
//             guess: &str,
//             features: impl IntoIterator<Item = &'a str>,
//         ) {
//             macro_rules! upd_feat {
//                 ($c:ident, $f:ident, $weights:expr, $v:expr) => {{
//                     let c: &str = $c;
//                     let f: &str = $f;
//                     let w: f64 = $weights.get(c).copied().unwrap_or_default();
//                     let v: f64 = $v;
//                     let prev_total = self
//                         .totals
//                         .get(f)
//                         .and_then(|map| map.get(c))
//                         .copied()
//                         .unwrap_or_default();
//                     let ts: u64 = self
//                         .timestamps
//                         .get(f)
//                         .and_then(|map| map.get(c))
//                         .copied()
//                         .unwrap_or_default();
//                     let plus_total = ((self.instances - ts) as f64) * w;
//                     self.totals
//                         .entry(f.into())
//                         .or_default()
//                         .insert(c.into(), prev_total + plus_total);
//                     self.timestamps
//                         .entry(f.into())
//                         .or_default()
//                         .insert(c.into(), self.instances);
//                     self.weights
//                         .entry(f.into())
//                         .or_default()
//                         .insert(c.into(), w + v);
//                 }};
//             }

//             self.instances += 1;
//             if truth == guess {
//                 return;
//             }
//             for f in features {
//                 upd_feat!(truth, f, self.weights.entry(f.into()).or_default(), 1.0);
//                 upd_feat!(guess, f, self.weights.entry(f.into()).or_default(), -1.0);
//             }
//         }

//         fn average_weights(&mut self) {
//             for (feat, weights) in self.weights.iter_mut() {
//                 let mut new_feat_weights = HashMap::with_capacity(weights.len());
//                 for (class, weight) in weights.iter() {
//                     // let param = (feat.clone(), class.clone());
//                     let mut total = self.totals[feat][class];
//                     let ts = self
//                         .timestamps
//                         .get(feat)
//                         .and_then(|map| map.get(class))
//                         .copied()
//                         .unwrap_or_default();
//                     total = total + (((self.instances - ts) as f64) * *weight);
//                     let averaged = total / (self.instances as f64);
//                     if averaged != 0.0 {
//                         new_feat_weights.insert(class.clone(), averaged);
//                     }
//                 }
//                 *weights = new_feat_weights;
//             }
//         }
//     }

//     pub struct PerceptronTagger {
//         model: AveragedPerceptron,
//         tag_dict: HashMap<String, String>,
//         classes: HashSet<String>,
//     }

//     const START: (&'static str, &'static str) = ("-START-", "-START2-");
//     const END: (&'static str, &'static str) = ("-END-", "-END2-");

//     impl PerceptronTagger {
//         pub fn new() -> Self {
//             Self {
//                 model: AveragedPerceptron::new(),
//                 tag_dict: HashMap::new(),
//                 classes: HashSet::new(),
//             }
//         }

//         pub fn new_trained() -> anyhow::Result<Self> {
//             let model = AveragedPerceptron::load()?;
//             Ok(Self {
//                 model,
//                 tag_dict: HashMap::new(),
//                 classes: HashSet::new(),
//             })
//         }

//         pub fn load_raw_and_train(&mut self) {
//             let raw_data = &super::conll2000::TRAIN_DATA;
//             // let mut training_data = Vec::with_capacity(raw_data.sentences.len());
//             // for sentence in raw_data.sentences.iter() {
//             //     let mut pair_vec = Vec::with_capacity(sentence.len());
//             //     for word_info in sentence {
//             //         pair_vec.push((word_info.word.clone(), word_info.chunk_tag.clone()))
//             //     }
//             //     training_data.push(pair_vec);
//             // }
//             #[cfg(debug_assertions)]
//             const TARGET: f64 = 0.90;
//             #[cfg(not(debug_assertions))]
//             const TARGET: f64 = 0.99;
//             debug!("Starting training...");
//             self.train(raw_data.sentences.clone(), TARGET);
//             #[cfg(not(debug_assertions))]
//             {
//                 debug!("Saving to disk...");
//                 self.model.save().unwrap();
//             }
//         }

//         pub fn tag<'a>(
//             &self,
//             tokens: &[&'a str],
//             return_conf: bool,
//             use_tagdict: bool,
//         ) -> Vec<(TagResult<'a>, Option<f64>)> {
//             let mut prev: String = START.0.into();
//             let mut prev2: String = START.1.into();
//             let mut output: Vec<(TagResult<'a>, Option<f64>)> = Vec::with_capacity(tokens.len());

//             let context: Vec<String> = {
//                 let mut v = Vec::with_capacity(tokens.len() + 4);
//                 v.push(START.0.into());
//                 v.push(START.1.into());
//                 v.extend(tokens.iter().copied().map(Self::normalize));
//                 v.push(END.0.into());
//                 v.push(END.1.into());
//                 v
//             };
//             for (i, word) in tokens.iter().enumerate() {
//                 let (chunk_tag_opt, conf_opt) = if use_tagdict {
//                     (self.tag_dict.get(*word), Some(1.0))
//                 } else {
//                     (None, None)
//                 };
//                 let (pos_tag, chunk_tag, conf_opt): (String, String, Option<f64>) =
//                     match chunk_tag_opt {
//                         Some(t) => ("TODO".into(), t.clone(), conf_opt),
//                         None => {
//                             let features = self.get_features(i, word, &context, &prev, &prev2);
//                             let (tag, conf_opt) = self.model.predict(&features, return_conf);
//                             ("TODO".into(), tag.into(), conf_opt)
//                         }
//                     };
//                 let tr = TagResult {
//                     word: word,
//                     // pos_tag,
//                     chunk_tag: chunk_tag.clone(),
//                 };
//                 output.push((tr, conf_opt));
//                 prev2 = prev;
//                 prev = chunk_tag;
//             }
//             output
//         }

//         fn train(&mut self, mut sentences: Vec<Vec<WordInfo>>, target_accuracy: f64) {
//             assert!(target_accuracy >= 0.0 && target_accuracy < 1.0);
//             use rand::seq::SliceRandom;
//             let mut rng = rand::thread_rng();

//             debug!("Starting to train model");

//             self.make_tag_dict(&sentences);
//             self.model.classes = self.classes.clone();
//             for i in 0.. {
//                 let mut c = 0;
//                 let mut n = 0;
//                 for sentence in sentences.iter() {
//                     let mut words = Vec::with_capacity(sentence.len());
//                     let mut tags = Vec::with_capacity(sentence.len());
//                     for word_info in sentence.iter() {
//                         words.push(word_info.word.as_str());
//                         tags.push(word_info.chunk_tag.as_str());
//                     }

//                     let mut prev: String = START.0.into();
//                     let mut prev2: String = START.1.into();
//                     let context: Vec<String> = {
//                         let mut v = Vec::with_capacity(words.len() + 4);
//                         v.push(START.0.into());
//                         v.push(START.1.into());
//                         v.extend(words.iter().copied().map(Self::normalize));
//                         v.push(END.0.into());
//                         v.push(END.1.into());
//                         v
//                     };
//                     for (i, word) in words.iter().copied().enumerate() {
//                         let guess: String = match self.tag_dict.get(word) {
//                             Some(g) => g.clone(),
//                             None => {
//                                 let feats = self.get_features(i, word, &context, &prev, &prev2);
//                                 let (guess, _) = self.model.predict(&feats, false);
//                                 let guess: String = guess.into();
//                                 self.model.update(
//                                     tags[i],
//                                     &guess,
//                                     feats.keys().map(String::as_str),
//                                 );
//                                 guess
//                             }
//                         };
//                         if guess.as_str() == tags[i] {
//                             c += 1;
//                         }
//                         n += 1;
//                         prev2 = prev;
//                         prev = guess;
//                     }
//                 }
//                 sentences.shuffle(&mut rng);
//                 let acc = (c as f64) / (n as f64);
//                 debug!(
//                     "Training iteration {} completed ({}/{} = {:.2}%)",
//                     i + 1,
//                     c,
//                     n,
//                     acc * 100.0,
//                 );
//                 if acc >= target_accuracy {
//                     break;
//                 }
//             }

//             self.model.average_weights();
//         }

//         fn normalize(word: &str) -> String {
//             if word.contains("-") && !word.starts_with('-') {
//                 "!HYPHEN".into()
//             } else if word.chars().all(|c| c.is_ascii_digit()) && word.len() == 4 {
//                 "!YEAR".into()
//             } else if word.chars().next().map(|c| c.is_ascii_digit()) == Some(true) {
//                 "!DIGITS".into()
//             } else {
//                 word.to_lowercase().into()
//             }
//         }

//         fn get_features(
//             &self,
//             i: usize,
//             word: &str,
//             context: &[String],
//             prev: &str,
//             prev2: &str,
//         ) -> HashMap<String, f64> {
//             fn add_by_key(features: &mut HashMap<String, f64>, key: String) {
//                 let slot: &mut f64 = features.entry(key).or_default();
//                 *slot = *slot + 1.0;
//             }

//             let mut features = HashMap::new();

//             macro_rules! add {
//                 ($label:expr) => {
//                     add_by_key(&mut features, format!("{}", $label));
//                 };
//                 ($label:expr, $part1:expr) => {
//                     add_by_key(&mut features, format!("{} {}", $label, $part1));
//                 };
//                 ($label:expr, $part1:expr, $part2:expr) => {
//                     add_by_key(&mut features, format!("{} {} {}", $label, $part1, $part2));
//                 };
//             }

//             fn suffix(s: &str) -> &str {
//                 &s[s.len().saturating_sub(3).max(0)..]
//             }

//             let i = i + 2;
//             add!("bias");
//             add!("i suffix", suffix(&word));
//             add!("i pref1", &word[0..]);
//             add!("i-1 tag", prev);
//             add!("i-2 tag", prev2);
//             add!("i tag+i-2 tag", prev, prev2);
//             add!("i word", &context[i]);
//             add!("i-1 tag+i word", prev, &context[i]);
//             add!("i-1 word", &context[i - 1]);
//             add!("i-1 suffix", suffix(&context[i - 1]));
//             add!("i-2 word", &context[i - 2]);
//             add!("i+1 word", &context[i + 1]);
//             add!("i+1 suffix", suffix(&context[i + 1]));
//             add!("i+2 word", (&context[i + 2]));
//             features
//         }

//         fn make_tag_dict(&mut self, sentences: &[Vec<WordInfo>]) {
//             let mut counts: HashMap<String, HashMap<String, u64>> = HashMap::new();
//             for sentence in sentences {
//                 for word_info in sentence {
//                     let slot = counts
//                         .entry(word_info.word.clone())
//                         .or_default()
//                         .entry(word_info.chunk_tag.clone())
//                         .or_default();
//                     *slot += 1;
//                     self.classes.insert(word_info.chunk_tag.clone());
//                 }
//             }
//             let freq_thresh = 20;
//             let ambig_thresh: f64 = 0.97;
//             for (word, tag_freqs) in counts {
//                 let n: u64 = tag_freqs.values().copied().sum();
//                 let (tag, mode) = tag_freqs
//                     .into_iter()
//                     .max_by_key(|(_, x)| *x)
//                     .unwrap_or_default();
//                 if n < freq_thresh {
//                     continue;
//                 }
//                 let rarity: f64 = (mode as f64) / (n as f64);
//                 if rarity < ambig_thresh {
//                     continue;
//                 }
//                 self.tag_dict.insert(word, tag);
//             }
//         }
//     }

//     // #[test]
//     // fn test_train_tagger() {
//     //     let trained = PerceptronTagger::new_trained();
//     //     assert!(false);
//     // }
// }
