---
composure: minor
---

# Linked Roles, onboarding, stage instances and lobbies Endpoints

Four new `BotClient` groups, in `full`:

- `rest-role-connections`: `get_role_connection_metadata`, `update_role_connection_metadata` (`types::ApplicationRoleConnectionMetadata`), for Linked Roles
- `rest-onboarding`: `get_guild_onboarding`, `modify_guild_onboarding` (`ModifyOnboarding`), `get_guild_welcome_screen`, `modify_guild_welcome_screen` (`ModifyWelcomeScreen`)
- `rest-stage-instances`: `create_stage_instance`, `get_stage_instance`, `modify_stage_instance`, `delete_stage_instance`
- `rest-lobbies`: lobbies (`create_lobby`, `create_or_join_lobby`, `get_lobby`, `modify_lobby`, `delete_lobby`, `link_lobby_channel`), members (`add_lobby_member`, `remove_lobby_member`, `bulk_update_lobby_members`, `leave_lobby`, guild invites) and messages (`get_lobby_messages`, `send_lobby_message`, `update_lobby_message_moderation_metadata`); `LobbyMessage` gains `moderation_metadata`
