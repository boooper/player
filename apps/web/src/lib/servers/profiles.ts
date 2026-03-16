import type {
  PlexAuthPollResultPayload,
  PlexAuthStartPayload,
  ProfileDraftPayload,
  ProfilePayload,
} from './types';
import { invoke } from './shared';

export async function fetchProfiles(): Promise<ProfilePayload[]> {
  return invoke<ProfilePayload[]>('get_profiles');
}

export async function createProfile(data: ProfileDraftPayload): Promise<ProfilePayload> {
  return invoke<ProfilePayload>('create_profile', { data });
}

export async function updateProfile(id: number, data: ProfileDraftPayload): Promise<ProfilePayload> {
  return invoke<ProfilePayload>('update_profile', { id, data });
}

export async function deleteProfile(id: number): Promise<void> {
  await invoke('delete_profile', { id });
}

export async function activateProfile(id: number): Promise<void> {
  await invoke('activate_profile', { id });
}

export async function plexBeginAuth(): Promise<PlexAuthStartPayload> {
  return invoke<PlexAuthStartPayload>('plex_begin_auth');
}

export async function plexPollAuth(pinId: number): Promise<PlexAuthPollResultPayload> {
  return invoke<PlexAuthPollResultPayload>('plex_poll_auth', { pinId });
}
