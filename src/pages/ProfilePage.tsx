import { BadgeCheck, BriefcaseBusiness, FolderKanban, GraduationCap, Plus, Save, ShieldCheck, Trash2, UserRound } from "lucide-react";
import { Children, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { Button, Field, InlineNotice, Input, PageHeader, SurfaceCard, TechLabel } from "../components/ui";
import { api } from "../lib/api";
import type { Profile, ProfileCertification, ProfileEducation, ProfileExperience, ProfileLanguage, ProfileProject, ProfileSkill } from "../types";

const emptyProfile: Profile = {
  fullName: "",
  experiences: [],
  education: [],
  projects: [],
  skills: [],
  languages: [],
  certifications: [],
};

const blankExperience = (): ProfileExperience => ({ company: "", title: "", current: false });
const blankEducation = (): ProfileEducation => ({ institution: "" });
const blankProject = (): ProfileProject => ({ name: "" });
const blankSkill = (): ProfileSkill => ({ name: "" });
const blankLanguage = (): ProfileLanguage => ({ name: "" });
const blankCertification = (): ProfileCertification => ({ name: "" });

export function ProfilePage() {
  const [profile, setProfile] = useState<Profile>(emptyProfile);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string>();
  const [error, setError] = useState<string>();

  useEffect(() => {
    api.profile().then((next) => setProfile({ ...emptyProfile, ...next })).catch((cause) => setError(String(cause))).finally(() => setLoading(false));
  }, []);

  const set = <K extends keyof Profile>(key: K, value: Profile[K]) => setProfile((current) => ({ ...current, [key]: value }));
  const save = async () => {
    setBusy(true); setNotice(undefined); setError(undefined);
    try { setProfile(await api.saveProfile(profile)); setNotice("Profile saved locally. Personal identifiers are not sent to matching by default."); }
    catch (cause) { setError(String(cause)); }
    finally { setBusy(false); }
  };
  const updateExperience = (index: number, value: ProfileExperience) => set("experiences", profile.experiences.map((item, itemIndex) => itemIndex === index ? value : item));
  const updateEducation = (index: number, value: ProfileEducation) => set("education", profile.education.map((item, itemIndex) => itemIndex === index ? value : item));
  const updateProject = (index: number, value: ProfileProject) => set("projects", profile.projects.map((item, itemIndex) => itemIndex === index ? value : item));
  const updateSkill = (index: number, value: ProfileSkill) => set("skills", profile.skills.map((item, itemIndex) => itemIndex === index ? value : item));
  const updateLanguage = (index: number, value: ProfileLanguage) => set("languages", profile.languages.map((item, itemIndex) => itemIndex === index ? value : item));
  const updateCertification = (index: number, value: ProfileCertification) => set("certifications", profile.certifications.map((item, itemIndex) => itemIndex === index ? value : item));

  if (loading) return <div className="page profile-loading">Loading your profile…</div>;

  return <div className="page">
    <PageHeader eyebrow="Your professional library" title="Profile" description="Keep your personal story, experience, and projects in one local workspace." actions={<Button variant="primary" icon={Save} disabled={busy} onClick={save}>{busy ? "Saving…" : "Save profile"}</Button>} />
    {notice && <InlineNotice tone="success" title="Saved">{notice}</InlineNotice>}
    {error && <InlineNotice tone="error" title="Could not save">{error}</InlineNotice>}
    <div className="profile-layout">
      <div className="profile-main">
        <SurfaceCard className="profile-section"><SectionHeading icon={UserRound} label="Identity" title="Personal details" description="Stored on this computer for your profile and future CV documents." /><div className="form-grid"><Field label="Full name"><Input value={profile.fullName} onChange={(event) => set("fullName", event.target.value)} placeholder="Your full name" /></Field><Field label="Preferred name"><Input value={profile.preferredName ?? ""} onChange={(event) => set("preferredName", event.target.value)} placeholder="What should we call you?" /></Field><Field label="Email"><Input type="email" value={profile.email ?? ""} onChange={(event) => set("email", event.target.value)} placeholder="you@example.com" /></Field><Field label="Phone"><Input value={profile.phone ?? ""} onChange={(event) => set("phone", event.target.value)} placeholder="Optional" /></Field><Field label="City / location"><Input value={profile.location ?? ""} onChange={(event) => set("location", event.target.value)} placeholder="Montevideo" /></Field><Field label="Country"><Input value={profile.country ?? ""} onChange={(event) => set("country", event.target.value)} placeholder="Uruguay" /></Field><Field label="Address"><Input value={profile.address ?? ""} onChange={(event) => set("address", event.target.value)} placeholder="Optional" /></Field><Field label="Availability"><Input value={profile.availability ?? ""} onChange={(event) => set("availability", event.target.value)} placeholder="Available from…" /></Field><Field label="LinkedIn URL"><Input value={profile.linkedinUrl ?? ""} onChange={(event) => set("linkedinUrl", event.target.value)} placeholder="https://linkedin.com/in/…" /></Field><Field label="GitHub URL"><Input value={profile.githubUrl ?? ""} onChange={(event) => set("githubUrl", event.target.value)} placeholder="https://github.com/…" /></Field><Field label="Portfolio URL"><Input value={profile.portfolioUrl ?? ""} onChange={(event) => set("portfolioUrl", event.target.value)} placeholder="https://…" /></Field><Field label="Work authorization"><Input value={profile.workAuthorization ?? ""} onChange={(event) => set("workAuthorization", event.target.value)} placeholder="Optional" /></Field></div></SurfaceCard>
        <SurfaceCard className="profile-section"><SectionHeading icon={BadgeCheck} label="Positioning" title="Professional summary" description="The short version of the value you bring to a team." /><div className="form-grid"><Field label="Headline" className="form-grid__wide"><Input value={profile.headline ?? ""} onChange={(event) => set("headline", event.target.value)} placeholder="Senior QA Engineer · Automation · Developer Experience" /></Field><Field label="Summary" className="form-grid__wide"><textarea className="input profile-textarea" value={profile.summary ?? ""} onChange={(event) => set("summary", event.target.value)} placeholder="A concise overview of your experience, strengths, and goals." rows={6} /></Field></div></SurfaceCard>
        <CollectionSection icon={BriefcaseBusiness} label="Experience" title="Work history" description="Add roles in the order you want to present them." onAdd={() => set("experiences", [...profile.experiences, blankExperience()])}>
          {profile.experiences.map((item, index) => <SurfaceCard className="profile-entry" key={item.id ?? `experience-${index}`}><EntryHeader title={item.title || "New role"} subtitle={item.company} onRemove={() => set("experiences", profile.experiences.filter((_, itemIndex) => itemIndex !== index))} /><div className="form-grid form-grid--three"><Field label="Company"><Input value={item.company} onChange={(event) => updateExperience(index, { ...item, company: event.target.value })} placeholder="Company" /></Field><Field label="Title"><Input value={item.title} onChange={(event) => updateExperience(index, { ...item, title: event.target.value })} placeholder="Role title" /></Field><Field label="Location"><Input value={item.location ?? ""} onChange={(event) => updateExperience(index, { ...item, location: event.target.value })} placeholder="Remote / city" /></Field><Field label="Start date"><Input type="month" value={item.startDate ?? ""} onChange={(event) => updateExperience(index, { ...item, startDate: event.target.value })} /></Field><Field label="End date"><Input type="month" value={item.endDate ?? ""} onChange={(event) => updateExperience(index, { ...item, endDate: event.target.value })} disabled={item.current} /></Field><label className="profile-checkbox"><input type="checkbox" checked={item.current} onChange={(event) => updateExperience(index, { ...item, current: event.target.checked, endDate: event.target.checked ? undefined : item.endDate })} /><span>Current role</span></label><Field label="Description" className="form-grid__wide"><textarea className="input profile-textarea" value={item.description ?? ""} onChange={(event) => updateExperience(index, { ...item, description: event.target.value })} placeholder="Responsibilities, achievements, and outcomes." rows={4} /></Field></div></SurfaceCard>)}
        </CollectionSection>
      </div>
      <aside className="profile-side">
        <SurfaceCard className="profile-privacy"><span className="profile-privacy__icon"><ShieldCheck size={20} /></span><TechLabel>Local & private</TechLabel><h2>Keep the full story here.</h2><p>Personal details are saved in your local SQLite database. When matching is added, Rolebit will build an explicit, minimized payload instead of sending this entire profile automatically.</p></SurfaceCard>
        <CollectionSection icon={GraduationCap} label="Education" title="Education" onAdd={() => set("education", [...profile.education, blankEducation()])} compact>
          {profile.education.map((item, index) => <SurfaceCard className="profile-entry" key={item.id ?? `education-${index}`}><EntryHeader title={item.institution || "New education"} subtitle={item.degree} onRemove={() => set("education", profile.education.filter((_, itemIndex) => itemIndex !== index))} /><div className="profile-stack"><Field label="Institution"><Input value={item.institution} onChange={(event) => updateEducation(index, { ...item, institution: event.target.value })} placeholder="University or program" /></Field><Field label="Degree"><Input value={item.degree ?? ""} onChange={(event) => updateEducation(index, { ...item, degree: event.target.value })} placeholder="Degree / certification" /></Field><Field label="Field of study"><Input value={item.fieldOfStudy ?? ""} onChange={(event) => updateEducation(index, { ...item, fieldOfStudy: event.target.value })} placeholder="Computer Science" /></Field><div className="form-grid"><Field label="Start"><Input type="month" value={item.startDate ?? ""} onChange={(event) => updateEducation(index, { ...item, startDate: event.target.value })} /></Field><Field label="End"><Input type="month" value={item.endDate ?? ""} onChange={(event) => updateEducation(index, { ...item, endDate: event.target.value })} /></Field></div></div></SurfaceCard>)}
        </CollectionSection>
        <CollectionSection icon={FolderKanban} label="Projects" title="Selected projects" onAdd={() => set("projects", [...profile.projects, blankProject()])} compact>
          {profile.projects.map((item, index) => <SurfaceCard className="profile-entry" key={item.id ?? `project-${index}`}><EntryHeader title={item.name || "New project"} subtitle={item.role} onRemove={() => set("projects", profile.projects.filter((_, itemIndex) => itemIndex !== index))} /><div className="profile-stack"><Field label="Name"><Input value={item.name} onChange={(event) => updateProject(index, { ...item, name: event.target.value })} placeholder="Project name" /></Field><Field label="Role"><Input value={item.role ?? ""} onChange={(event) => updateProject(index, { ...item, role: event.target.value })} placeholder="Your contribution" /></Field><Field label="URL"><Input value={item.url ?? ""} onChange={(event) => updateProject(index, { ...item, url: event.target.value })} placeholder="https://…" /></Field><Field label="Technologies"><Input value={item.technologies ?? ""} onChange={(event) => updateProject(index, { ...item, technologies: event.target.value })} placeholder="Rust, React, SQLite" /></Field><Field label="Description"><textarea className="input profile-textarea" value={item.description ?? ""} onChange={(event) => updateProject(index, { ...item, description: event.target.value })} rows={4} /></Field></div></SurfaceCard>)}
        </CollectionSection>
        <CollectionSection icon={BadgeCheck} label="Skills" title="Skills" onAdd={() => set("skills", [...profile.skills, blankSkill()])} compact>
          {profile.skills.map((item, index) => <div className="profile-skill" key={item.id ?? `skill-${index}`}><Input value={item.name} onChange={(event) => updateSkill(index, { ...item, name: event.target.value })} placeholder="Skill" /><Input value={item.category ?? ""} onChange={(event) => updateSkill(index, { ...item, category: event.target.value })} placeholder="Category" /><Button variant="ghost" icon={Trash2} aria-label={`Remove skill ${index + 1}`} onClick={() => set("skills", profile.skills.filter((_, itemIndex) => itemIndex !== index))} /></div>)}
        </CollectionSection>
        <CollectionSection icon={UserRound} label="Languages" title="Languages" onAdd={() => set("languages", [...profile.languages, blankLanguage()])} compact>
          {profile.languages.map((item, index) => <div className="profile-skill" key={item.id ?? `language-${index}`}><Input value={item.name} onChange={(event) => updateLanguage(index, { ...item, name: event.target.value })} placeholder="Language" /><Input value={item.proficiency ?? ""} onChange={(event) => updateLanguage(index, { ...item, proficiency: event.target.value })} placeholder="Proficiency" /><Button variant="ghost" icon={Trash2} aria-label={`Remove language ${index + 1}`} onClick={() => set("languages", profile.languages.filter((_, itemIndex) => itemIndex !== index))} /></div>)}
        </CollectionSection>
        <CollectionSection icon={BadgeCheck} label="Credentials" title="Certifications" onAdd={() => set("certifications", [...profile.certifications, blankCertification()])} compact>
          {profile.certifications.map((item, index) => <SurfaceCard className="profile-entry" key={item.id ?? `certification-${index}`}><EntryHeader title={item.name || "New certification"} subtitle={item.issuer} onRemove={() => set("certifications", profile.certifications.filter((_, itemIndex) => itemIndex !== index))} /><div className="profile-stack"><Field label="Name"><Input value={item.name} onChange={(event) => updateCertification(index, { ...item, name: event.target.value })} placeholder="Certification" /></Field><Field label="Issuer"><Input value={item.issuer ?? ""} onChange={(event) => updateCertification(index, { ...item, issuer: event.target.value })} placeholder="Issuing organization" /></Field><Field label="Issue date"><Input type="month" value={item.issueDate ?? ""} onChange={(event) => updateCertification(index, { ...item, issueDate: event.target.value })} /></Field><Field label="Credential URL"><Input value={item.credentialUrl ?? ""} onChange={(event) => updateCertification(index, { ...item, credentialUrl: event.target.value })} placeholder="https://…" /></Field></div></SurfaceCard>)}
        </CollectionSection>
      </aside>
    </div>
  </div>;
}

function SectionHeading({ icon: Icon, label, title, description }: { icon: typeof UserRound; label: string; title: string; description?: string }) {
  return <div className="profile-section__heading"><span><Icon size={19} /></span><div><TechLabel>{label}</TechLabel><h2>{title}</h2>{description && <p>{description}</p>}</div></div>;
}

function CollectionSection({ icon: Icon, label, title, description, onAdd, children, compact = false }: { icon: typeof UserRound; label: string; title: string; description?: string; onAdd: () => void; children: ReactNode; compact?: boolean }) {
  return <section className={`profile-collection ${compact ? "profile-collection--compact" : ""}`}><div className="profile-collection__heading"><div><SectionHeading icon={Icon} label={label} title={title} description={description} /></div><Button variant="secondary" icon={Plus} onClick={onAdd}>Add</Button></div>{Children.count(children) ? children : <p className="profile-empty">Nothing added yet.</p>}</section>;
}

function EntryHeader({ title, subtitle, onRemove }: { title: string; subtitle?: string; onRemove: () => void }) {
  return <div className="profile-entry__header"><div><strong>{title}</strong>{subtitle && <span>{subtitle}</span>}</div><Button variant="ghost" icon={Trash2} aria-label={`Remove ${title}`} onClick={onRemove}>Remove</Button></div>;
}
