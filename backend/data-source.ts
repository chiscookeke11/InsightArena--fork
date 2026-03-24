import { config } from 'dotenv';

// Load environment variables for TypeORM CLI FIRST
config();

import dataSource from './src/config/typeorm.config';

export default dataSource;
