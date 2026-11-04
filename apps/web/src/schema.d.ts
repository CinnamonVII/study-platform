/* Generated from schemas/snapshot-v2.json. Run python -m engine.cli schema && node scripts/web-schema.mjs */

export type SchemaVersion = "2.0";
export type Id = string;
export type Title = string;
export type Kind = "cours" | "devoir" | "exercice" | "correction" | "fiche" | "methode" | "annale";
export type Level = (1 | 2) | null;
export type ProvenanceReason = string;
export type Subject = string;
export type Chapter = string;
export type ModifiedAt = string;
export type ContentHash = string;
export type Status = "pending" | "validated" | "rejected";
export type DocumentId = string;
export type Page = number | null;
export type Region = number[] | null;
export type Quote = string;
export type Sources = Reference[];
export type Text = string;
export type Original = string;
export type OriginUrl = string;
export type Private = boolean;
export type Deleted = boolean;
export type Message = string;
export type Page1 = number | null;
export type Region1 = number[] | null;
export type Resolved = boolean;
export type Uncertainties = Uncertainty[];
export type Manual = boolean;
export type Derived = boolean;
export type Id1 = string;
export type Title1 = string;
export type Subject1 = string;
export type Chapter1 = string;
export type Level1 = 2;
/**
 * @minItems 1
 */
export type Blocks = [Block, ...Block[]];
export type Kind1 =
  "definition" | "explanation" | "steps" | "mechanism" | "comparison" | "timeline" | "warning" | "formula";
export type Title2 = string;
export type Text1 = string;
export type Items = string[];
export type Columns = string[];
export type Rows = string[][];
export type Sources1 = Reference[];
/**
 * @minItems 1
 */
export type Takeaways = [string, ...string[]];
/**
 * @minItems 1
 */
export type Recall = [string, ...string[]];
/**
 * @minItems 1
 */
export type Sources2 = [Reference, ...Reference[]];
export type Status1 = "pending" | "validated" | "rejected";
export type CreatedAt = string;
export type UpdatedAt = string;
export type GeneratorVersion = string;
export type DependencyHash = string;
export type NotionUrl = string;
export type Manual1 = boolean;
export type Stale = boolean;
export type Private1 = boolean;
export type Id2 = string;
export type ConceptKey = string;
export type Subject2 = string;
export type Chapter2 = string;
export type Type = "short" | "mcq" | "open" | "math" | "method" | "flashcard";
export type Cognitive = "memory" | "understanding" | "application" | "reasoning";
export type Prompt = string;
export type Answer = string;
export type Explanation = string;
export type Choices = string[];
export type CorrectIndices = number[];
export type Traps = string[];
export type Steps = string[];
export type Rubric = string[];
export type Difficulty = number;
/**
 * @minItems 1
 */
export type Sources3 = [Reference, ...Reference[]];
export type FicheIds = string[];
export type Status2 = "pending" | "validated" | "rejected";
export type CreatedAt1 = string;
export type UpdatedAt1 = string;
export type GeneratorVersion1 = string;
export type DependencyHash1 = string;
export type Manual2 = boolean;
export type Stale1 = boolean;
export type Private2 = boolean;
export type QuestionId = string;
export type Due = string;
export type Interval = number;
export type Ease = number;
export type Repetitions = number;
export type LastGrade = number | null;
export type Reports = {
  [k: string]: unknown;
}[];
export type LastSuccess = string | null;
export type Demo = boolean;

export interface Snapshot {
  schema_version: SchemaVersion;
  documents: Documents;
  fiches: Fiches;
  questions: Questions;
  reviews: Reviews;
  proposals: Proposals;
  jobs: Jobs;
  reports: Reports;
  last_success: LastSuccess;
  demo: Demo;
}
export interface Documents {
  [k: string]: Document;
}
export interface Document {
  id: Id;
  title: Title;
  kind: Kind;
  level: Level;
  provenance_reason: ProvenanceReason;
  subject: Subject;
  chapter: Chapter;
  modified_at: ModifiedAt;
  content_hash: ContentHash;
  status: Status;
  sources: Sources;
  text: Text;
  original: Original;
  origin_url: OriginUrl;
  private: Private;
  deleted: Deleted;
  uncertainties: Uncertainties;
  manual: Manual;
  derived: Derived;
}
export interface Reference {
  document_id: DocumentId;
  page: Page;
  region: Region;
  quote: Quote;
}
export interface Uncertainty {
  message: Message;
  page: Page1;
  region: Region1;
  resolved: Resolved;
}
export interface Fiches {
  [k: string]: Fiche;
}
export interface Fiche {
  id: Id1;
  title: Title1;
  subject: Subject1;
  chapter: Chapter1;
  level: Level1;
  blocks: Blocks;
  takeaways: Takeaways;
  recall: Recall;
  sources: Sources2;
  status: Status1;
  created_at: CreatedAt;
  updated_at: UpdatedAt;
  generator_version: GeneratorVersion;
  dependency_hash: DependencyHash;
  notion_url: NotionUrl;
  manual: Manual1;
  stale: Stale;
  private: Private1;
}
export interface Block {
  kind: Kind1;
  title: Title2;
  text: Text1;
  items: Items;
  columns: Columns;
  rows: Rows;
  sources: Sources1;
}
export interface Questions {
  [k: string]: Question;
}
export interface Question {
  id: Id2;
  concept_key: ConceptKey;
  subject: Subject2;
  chapter: Chapter2;
  type: Type;
  cognitive: Cognitive;
  prompt: Prompt;
  answer: Answer;
  explanation: Explanation;
  choices: Choices;
  correct_indices: CorrectIndices;
  traps: Traps;
  steps: Steps;
  rubric: Rubric;
  difficulty: Difficulty;
  sources: Sources3;
  fiche_ids: FicheIds;
  status: Status2;
  created_at: CreatedAt1;
  updated_at: UpdatedAt1;
  generator_version: GeneratorVersion1;
  dependency_hash: DependencyHash1;
  manual: Manual2;
  stale: Stale1;
  private: Private2;
}
export interface Reviews {
  [k: string]: Review;
}
export interface Review {
  question_id: QuestionId;
  due: Due;
  interval: Interval;
  ease: Ease;
  repetitions: Repetitions;
  last_grade: LastGrade;
}
export interface Proposals {
  [k: string]: {
    [k: string]: unknown;
  };
}
export interface Jobs {
  [k: string]: {
    [k: string]: unknown;
  };
}
