import type { Profile } from "../types";

function hasText(...values: Array<string | undefined>) {
  return values.some((value) => Boolean(value?.trim()));
}

export function validateProfile(profile: Profile): string | undefined {
  if (!profile.fullName.trim()) return "Your full name is required.";

  for (const [index, item] of profile.experiences.entries()) {
    if (!item.current && !hasText(item.company, item.title, item.location, item.startDate, item.endDate, item.description)) continue;
    if (!item.company.trim()) return `Experience entry ${index + 1} needs a company.`;
    if (!item.title.trim()) return `Experience entry ${index + 1} needs a title.`;
  }
  for (const [index, item] of profile.education.entries()) {
    if (!hasText(item.institution, item.degree, item.fieldOfStudy, item.location, item.startDate, item.endDate, item.description)) continue;
    if (!item.institution.trim()) return `Education entry ${index + 1} needs an institution.`;
  }
  for (const [index, item] of profile.projects.entries()) {
    if (!hasText(item.name, item.role, item.url, item.technologies, item.description)) continue;
    if (!item.name.trim()) return `Project entry ${index + 1} needs a name.`;
  }
  for (const [index, item] of profile.skills.entries()) {
    if (!hasText(item.name, item.category, item.level)) continue;
    if (!item.name.trim()) return `Skill entry ${index + 1} needs a name.`;
  }
  for (const [index, item] of profile.languages.entries()) {
    if (!hasText(item.name, item.proficiency)) continue;
    if (!item.name.trim()) return `Language entry ${index + 1} needs a language.`;
  }
  for (const [index, item] of profile.certifications.entries()) {
    if (!hasText(item.name, item.issuer, item.issueDate, item.credentialUrl)) continue;
    if (!item.name.trim()) return `Certification entry ${index + 1} needs a name.`;
  }
  return undefined;
}
