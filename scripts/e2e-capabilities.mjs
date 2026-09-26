export function seedE2ECapabilityConfigurations(runCompose) {
  runCompose([
    "exec",
    "-T",
    "postgres",
    "psql",
    "-U",
    "analytics",
    "-d",
    "analytics",
    "-v",
    "ON_ERROR_STOP=1",
    "-c",
    `WITH sites(site_id) AS (
      VALUES ('site_playground'), ('site_alpha'), ('site_beta'), ('site_unknown')
    ), capabilities(capability_id) AS (
      VALUES
        ('page_views'), ('browser_context'), ('anonymous_visitors'), ('sessions'),
        ('dimensions'), ('custom_events'), ('web_vitals'), ('conversions'), ('funnels'), ('geo')
    ), documents AS (
      SELECT sites.site_id, NOW() AS updated_at,
        jsonb_object_agg(capabilities.capability_id,
          jsonb_build_object('enabled', TRUE, 'settings', '{}'::jsonb)
        ) AS capabilities
      FROM sites CROSS JOIN capabilities
      GROUP BY sites.site_id
    )
    INSERT INTO site_capability_configurations (site_id, version, updated_at, document)
    SELECT site_id, 1, updated_at, jsonb_build_object(
      'schema_version', 1,
      'site_id', site_id,
      'version', 1,
      'updated_at', to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"'),
      'capabilities', capabilities,
      'consent_policy', 'required',
      'privacy_constraints', jsonb_build_array('no_ip_persistence', 'no_fingerprinting', 'consent_required')
    )
    FROM documents
    ON CONFLICT (site_id) DO NOTHING;

    INSERT INTO site_capability_activation_windows (site_id, capability_id, enabled_since)
    SELECT configurations.site_id, capability.key, '0001-01-01T00:00:00Z'::timestamptz
    FROM site_capability_configurations AS configurations
    CROSS JOIN LATERAL jsonb_each(configurations.document->'capabilities') AS capability(key, value)
    WHERE configurations.site_id IN ('site_playground', 'site_alpha', 'site_beta', 'site_unknown')
      AND capability.value->>'enabled' = 'true'
    ON CONFLICT (site_id, capability_id) DO NOTHING`,
  ]);
}
