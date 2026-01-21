-- Create extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- Users table (synced with Logto)
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    logto_user_id VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255),
    username VARCHAR(100),
    full_name VARCHAR(255),
    role VARCHAR(50) DEFAULT 'user' CHECK (role IN ('user', 'admin', 'moderator')),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true
);

-- Categories for issues
CREATE TABLE categories (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    icon VARCHAR(50),
    color VARCHAR(20),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    is_active BOOLEAN DEFAULT true
);

-- Chat sessions
CREATE TABLE chat_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    title VARCHAR(255),
    status VARCHAR(50) DEFAULT 'active' CHECK (status IN ('active', 'completed', 'archived')),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_message_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Chat messages
CREATE TABLE chat_messages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Reports extracted from chats
CREATE TABLE reports (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID NOT NULL REFERENCES chat_sessions(id),
    user_id UUID NOT NULL REFERENCES users(id),
    category_id UUID REFERENCES categories(id),
    
    -- Report details
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    
    -- Location data
    location_text TEXT,
    latitude DECIMAL(10, 8),
    longitude DECIMAL(11, 8),
    address TEXT,
    
    -- Time data
    incident_date TIMESTAMP WITH TIME ZONE,
    reported_date TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Status tracking
    status VARCHAR(50) DEFAULT 'submitted' CHECK (status IN (
        'draft', 'submitted', 'verified', 'in_progress', 
        'resolved', 'rejected', 'duplicate'
    )),
    
    -- Completeness
    is_complete BOOLEAN DEFAULT false,
    completeness_score DECIMAL(3, 2) DEFAULT 0.0,
    missing_fields JSONB DEFAULT '[]',
    
    -- NER extracted data
    entities JSONB DEFAULT '{}',
    
    -- Clustering
    cluster_id UUID,
    
    -- Media
    attachments JSONB DEFAULT '[]',
    
    -- Metadata
    metadata JSONB DEFAULT '{}',
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Report clusters (for grouping similar reports)
CREATE TABLE report_clusters (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255),
    description TEXT,
    category_id UUID REFERENCES categories(id),
    
    -- Clustering metadata
    centroid JSONB,
    report_count INTEGER DEFAULT 0,
    
    -- Location clustering
    center_latitude DECIMAL(10, 8),
    center_longitude DECIMAL(11, 8),
    radius_meters DECIMAL(10, 2),
    
    -- Time clustering
    earliest_incident TIMESTAMP WITH TIME ZONE,
    latest_incident TIMESTAMP WITH TIME ZONE,
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Tickets for tracking reports
CREATE TABLE tickets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    ticket_number VARCHAR(50) UNIQUE NOT NULL,
    report_id UUID NOT NULL REFERENCES reports(id),
    user_id UUID NOT NULL REFERENCES users(id),
    
    -- Ticket status
    status VARCHAR(50) DEFAULT 'open' CHECK (status IN (
        'open', 'assigned', 'in_progress', 'pending', 
        'resolved', 'closed', 'reopened'
    )),
    priority VARCHAR(20) DEFAULT 'medium' CHECK (priority IN ('low', 'medium', 'high', 'urgent')),
    
    -- Assignment
    assigned_to UUID REFERENCES users(id),
    assigned_at TIMESTAMP WITH TIME ZONE,
    
    -- Resolution
    resolution TEXT,
    resolved_at TIMESTAMP WITH TIME ZONE,
    resolved_by UUID REFERENCES users(id),
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Ticket comments
CREATE TABLE ticket_comments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    ticket_id UUID NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    comment TEXT NOT NULL,
    is_internal BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- API Keys for LLM providers
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    provider VARCHAR(50) NOT NULL CHECK (provider IN ('openrouter', 'openai', 'anthropic', 'custom')),
    api_key TEXT NOT NULL,
    base_url VARCHAR(255),
    is_active BOOLEAN DEFAULT true,
    usage_count BIGINT DEFAULT 0,
    last_used_at TIMESTAMP WITH TIME ZONE,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- System prompts
CREATE TABLE system_prompts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    prompt_type VARCHAR(50) NOT NULL CHECK (prompt_type IN (
        'chat_assistant', 'report_extraction', 'completeness_check', 
        'ner_extraction', 'clustering', 'summarization'
    )),
    prompt_text TEXT NOT NULL,
    variables JSONB DEFAULT '{}',
    is_active BOOLEAN DEFAULT true,
    version INTEGER DEFAULT 1,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Background jobs tracking
CREATE TABLE background_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_type VARCHAR(50) NOT NULL CHECK (job_type IN (
        'clustering', 'ner_processing', 'report_analysis', 'cleanup'
    )),
    status VARCHAR(20) DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'completed', 'failed')),
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_chat_sessions_user_id ON chat_sessions(user_id);
CREATE INDEX idx_chat_messages_session_id ON chat_messages(session_id);
CREATE INDEX idx_reports_user_id ON reports(user_id);
CREATE INDEX idx_reports_category_id ON reports(category_id);
CREATE INDEX idx_reports_status ON reports(status);
CREATE INDEX idx_reports_cluster_id ON reports(cluster_id);
CREATE INDEX idx_reports_latitude ON reports(latitude) WHERE latitude IS NOT NULL;
CREATE INDEX idx_reports_longitude ON reports(longitude) WHERE longitude IS NOT NULL;
CREATE INDEX idx_tickets_user_id ON tickets(user_id);
CREATE INDEX idx_tickets_status ON tickets(status);
CREATE INDEX idx_tickets_ticket_number ON tickets(ticket_number);

-- Full-text search
CREATE INDEX idx_reports_search ON reports USING gin(to_tsvector('indonesian', title || ' ' || description));
CREATE INDEX idx_categories_search ON categories USING gin(to_tsvector('indonesian', name || ' ' || COALESCE(description, '')));

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_categories_updated_at BEFORE UPDATE ON categories FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_chat_sessions_updated_at BEFORE UPDATE ON chat_sessions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_reports_updated_at BEFORE UPDATE ON reports FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_tickets_updated_at BEFORE UPDATE ON tickets FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert default categories
INSERT INTO categories (name, description, icon, color) VALUES
('Infrastruktur Jalan', 'Jalan rusak, lubang, retak, dll', 'road', '#FF6B6B'),
('Kebersihan', 'Sampah menumpuk, drainase tersumbat', 'trash', '#4ECDC4'),
('Penerangan', 'Lampu jalan mati atau rusak', 'lightbulb', '#FFE66D'),
('Air & Sanitasi', 'Masalah air bersih, saluran air', 'water', '#95E1D3'),
('Banjir', 'Genangan air, banjir', 'flood', '#3498DB'),
('Fasilitas Umum', 'Taman, bangku, halte rusak', 'facility', '#9B59B6'),
('Keamanan', 'Area gelap, tidak aman', 'security', '#E74C3C'),
('Lainnya', 'Laporan lain yang tidak masuk kategori', 'other', '#95A5A6');

-- Insert default system prompts
INSERT INTO system_prompts (name, prompt_type, prompt_text, variables) VALUES
('Chat Assistant', 'chat_assistant', 'Anda adalah asisten AI BalungPisah yang membantu warga melaporkan masalah infrastruktur dan layanan publik. Tugas Anda:
1. Tanyakan dengan ramah tentang masalah yang dihadapi
2. Gali informasi penting: lokasi spesifik, waktu kejadian, deskripsi detail
3. Minta foto jika memungkinkan
4. Bantu kategorikan masalah
5. Validasi kelengkapan laporan sebelum menyimpan

Gunakan bahasa Indonesia yang sopan dan mudah dipahami. Tunjukkan empati terhadap masalah warga.', '{}'),

('Report Extraction', 'report_extraction', 'Ekstrak informasi laporan dari percakapan berikut. Return JSON dengan struktur:
{
  "title": "judul singkat laporan",
  "description": "deskripsi lengkap",
  "location_text": "lokasi yang disebutkan",
  "category": "kategori yang sesuai",
  "incident_date": "tanggal kejadian jika disebutkan",
  "urgency": "low|medium|high"
}

Percakapan: {{conversation}}', '{"conversation": ""}'),

('Completeness Check', 'completeness_check', 'Periksa kelengkapan laporan berikut dan beri skor 0-1. Return JSON:
{
  "is_complete": true/false,
  "completeness_score": 0.0-1.0,
  "missing_fields": ["field1", "field2"],
  "suggestions": ["saran perbaikan"]
}

Required fields:
- Deskripsi jelas masalah
- Lokasi spesifik (nama jalan, kelurahan, atau koordinat)
- Waktu/tanggal kejadian (minimal perkiraan)
- Kategori masalah

Laporan: {{report}}', '{"report": ""}'),

('NER Extraction', 'ner_extraction', 'Ekstrak entitas dari teks laporan. Return JSON:
{
  "locations": ["lokasi1", "lokasi2"],
  "dates": ["tanggal1"],
  "organizations": ["instansi terkait"],
  "persons": ["nama orang jika ada"],
  "facilities": ["fasilitas yang disebutkan"]
}

Teks: {{text}}', '{"text": ""}'),

('Clustering Analysis', 'clustering', 'Analisis apakah laporan ini mirip dengan laporan lain dalam cluster. Return JSON:
{
  "similarity_score": 0.0-1.0,
  "is_similar": true/false,
  "reasoning": "alasan"
}

Laporan baru: {{new_report}}
Cluster existing: {{cluster_reports}}', '{"new_report": "", "cluster_reports": ""}');