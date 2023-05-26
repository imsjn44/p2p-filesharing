import React, { useContext } from 'react';
import { createStyles, Table, Text, Skeleton, ScrollArea, rem, Box, ActionIcon, Badge } from '@mantine/core';
import { IconDownload, IconSquareArrowLeft, IconSquareArrowRight } from '@tabler/icons-react';
import FileAdd from './FileAdd';
import { invoke } from '@tauri-apps/api/tauri';
import { emit, listen } from '@tauri-apps/api/event';
import { useState, useEffect } from 'react';
import { MyContext } from './Context';
import { useNavigate } from 'react-router-dom';


const useStyles = createStyles((theme) => ({
  progressBar: {
    '&:not(:first-of-type)': {
      borderLeft: `${rem(3)} solid ${theme.colorScheme === 'dark' ? theme.colors.dark[7] : theme.white
        }`,
    },
  },
}));



export default function RemoteFiles() {
  const navigate = useNavigate();

  const {  setTransfer, searchResults, setDownloadInfos } = useContext(MyContext);




  const handleDownloadClick = async (fileHash, chunks) => {

    const selectedFile = searchResults.find(elem => elem.fileHash === fileHash);
    // console.log("selected ", selectedFile); 
  
    // setTransfer((prev)=>[selectedFile, ...prev]);
    setTransfer((prev) => {
      for (const file of prev) {
        if (file.fileHash == selectedFile.fileHash )
          return [...prev]
      }
      return [selectedFile, ...prev]
    });
    navigate("/");
    await emit('file_info', fileHash);
    console.log("started download tracker");
    const eventName = `download_speed_${fileHash}`; 
    const unlisten = await listen(eventName, (event)=>{
      const payload = event.payload;
      setDownloadInfos((prev)=>{
        let temp = prev.filter(item => item.file_hash !== fileHash );
        return [...temp, payload]
      });
    })
    await emit('handle_download', {file_hash:fileHash, chunks:chunks});
    unlisten();
    console.log("just ended");
  }
  function formatBytes(bytes) {
    if (bytes === 0) return '0 B';
  
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
  
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }
  const rows = searchResults.length ? searchResults.map((row) => {
    return (
        <tr key={row.fileHash}>

          <td>
            {row.name}

          </td>
          <td>{formatBytes(row.fileSize)}</td>
          <td>{row.chunks}</td>
          <td>{row.fileHash}</td>

          <td>
            {row.seeds}
          </td>
          <td> <Box sx={{ display: 'flex', justifyContent: 'space-evenly' }}><ActionIcon onClick={() => { handleDownloadClick(row.fileHash, row.chunks) }} >

            <IconDownload /></ActionIcon></Box></td>


        </tr>
    );

  }

  ) : (<Skeleton height={100} width='391%' />)


  return (
    <>
      <Badge className='font-badge' style={{ fontSize: '25px', padding: '20px', marginTop: '20px' }} color="gray" variant="outline">Remote Files</Badge>

      <ScrollArea sx={{ marginTop: "20px" }}>
        <Table sx={{ minWidth: 800 }} verticalSpacing="xs">
          <thead>
            <tr className='font-header'>
              <th>Name</th>
              <th>Size</th>
              <th>Chunks</th>
              <th>File Hash</th>
              <th>Seeds</th>
            </tr>
          </thead>
          <tbody>{rows}</tbody>

        </Table>
      </ScrollArea>

      <FileAdd />

    </>
  );

}