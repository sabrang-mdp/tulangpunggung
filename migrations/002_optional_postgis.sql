-- Optional: Advanced Spatial Indexing with PostGIS
-- Run this ONLY if you need advanced geospatial features
-- and have PostGIS installed

-- Enable PostGIS extension
CREATE EXTENSION IF NOT EXISTS postgis;

-- Add geometry column
ALTER TABLE reports ADD COLUMN IF NOT EXISTS geom geometry(Point, 4326);

-- Create function to update geometry from lat/lon
CREATE OR REPLACE FUNCTION update_report_geometry()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.latitude IS NOT NULL AND NEW.longitude IS NOT NULL THEN
        NEW.geom = ST_SetSRID(ST_MakePoint(NEW.longitude::double precision, NEW.latitude::double precision), 4326);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger
DROP TRIGGER IF EXISTS trigger_update_report_geometry ON reports;
CREATE TRIGGER trigger_update_report_geometry
    BEFORE INSERT OR UPDATE ON reports
    FOR EACH ROW
    EXECUTE FUNCTION update_report_geometry();

-- Create spatial index
CREATE INDEX IF NOT EXISTS idx_reports_geom ON reports USING GIST(geom);

-- Update existing records
UPDATE reports 
SET geom = ST_SetSRID(ST_MakePoint(longitude::double precision, latitude::double precision), 4326)
WHERE latitude IS NOT NULL AND longitude IS NOT NULL;

-- Add geometry column to clusters
ALTER TABLE report_clusters ADD COLUMN IF NOT EXISTS geom geometry(Point, 4326);

-- Update cluster geometries
UPDATE report_clusters
SET geom = ST_SetSRID(ST_MakePoint(center_longitude::double precision, center_latitude::double precision), 4326)
WHERE center_latitude IS NOT NULL AND center_longitude IS NOT NULL;

-- Create spatial index for clusters
CREATE INDEX IF NOT EXISTS idx_clusters_geom ON report_clusters USING GIST(geom);

-- Helpful spatial queries you can now use:
-- 
-- 1. Find reports within 1km radius:
-- SELECT * FROM reports 
-- WHERE ST_DWithin(geom::geography, ST_SetSRID(ST_MakePoint(106.8456, -6.2088), 4326)::geography, 1000);
--
-- 2. Find nearest reports:
-- SELECT *, ST_Distance(geom::geography, ST_SetSRID(ST_MakePoint(106.8456, -6.2088), 4326)::geography) as distance
-- FROM reports
-- ORDER BY geom <-> ST_SetSRID(ST_MakePoint(106.8456, -6.2088), 4326)
-- LIMIT 10;
--
-- 3. Cluster reports by distance:
-- SELECT ST_ClusterKMeans(geom, 10) OVER () as cluster_id, * FROM reports;